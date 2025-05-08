use crate::{
    BBox, BLACK, CompiledLight, LightType, Linedef, Map, Pixel, Rasterizer, Ray, Sector,
    ShapeContext, Terrain, TerrainChunk, ValueContainer,
};
use noiselib::prelude::*;
use std::str::FromStr;
use theframework::prelude::*;
use uuid::Uuid;
use vek::Vec4;

#[inline(always)]
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

const BAYER_4X4: [[f32; 4]; 4] = [
    [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
    [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
    [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
    [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
];

#[derive(Debug, Clone, PartialEq)]
pub enum ShapeFXParam {
    /// Id, Name, Status, Value, Range
    Float(String, String, String, f32, std::ops::RangeInclusive<f32>),
    /// Id, Name, Status, Value, Range
    Int(String, String, String, i32, std::ops::RangeInclusive<i32>),
    /// Id, Name, Status, Value
    PaletteIndex(String, String, String, i32),
    /// Id, Name, Status, Options, Value
    Selector(String, String, String, Vec<String>, i32),
    /// Id, Name, Status, Value
    Color(String, String, String, TheColor),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShapeFXRole {
    // Material Group
    // These nodes get attached to geometry and produce pixel output
    MaterialGeometry,
    Gradient,
    Color,
    Outline,
    NoiseOverlay,
    Glow,
    Wood,
    // Sector and Linedef Group
    // These nodes get attached to geometry and control mesh creation
    // or produce rendering fx like lights, particles etc.
    LinedefGeometry,
    SectorGeometry,
    Flatten,
    // Render Group
    Render, // Main Render Node
    Fog,
    Sky,
    // FX Group
    Material,
    PointLight,
}

use ShapeFXRole::*;

// impl fmt::Display for ShapeFXRole {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let s = match self {
//             ShapeFXRole::MaterialGeometry => "Material Geometry",
//             ShapeFXRole::Gradient => "Gradient",
//             ShapeFXRole::Color => "Color",
//             ShapeFXRole::Outline => "Outline",
//             ShapeFXRole::NoiseOverlay => "Noise Overlay",
//             ShapeFXRole::Glow => "Glow",
//             ShapeFXRole::RegionGeometry => "Region Geometry",
//             ShapeFXRole::Flatten => "Flatten",
//             ShapeFXRole::Render => "Render",
//             ShapeFXRole::Lights => "Lights",
//             ShapeFXRole::Fog => "Fog",
//             ShapeFXRole::Sky => "Sky",
//         };
//         write!(f, "{}", s)
//     }
// }

impl FromStr for ShapeFXRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Material Geometry" => Ok(ShapeFXRole::MaterialGeometry),
            "Gradient" => Ok(ShapeFXRole::Gradient),
            "Color" => Ok(ShapeFXRole::Color),
            "Outline" => Ok(ShapeFXRole::Outline),
            "Noise Overlay" => Ok(ShapeFXRole::NoiseOverlay),
            "Glow" => Ok(ShapeFXRole::Glow),
            "Wood" => Ok(ShapeFXRole::Wood),
            "Sector Geometry" => Ok(ShapeFXRole::SectorGeometry),
            "Flatten" => Ok(ShapeFXRole::Flatten),
            "Render" => Ok(ShapeFXRole::Render),
            "Fog" => Ok(ShapeFXRole::Fog),
            "Sky" => Ok(ShapeFXRole::Sky),
            "Material" => Ok(ShapeFXRole::Material),
            "Point Light" => Ok(ShapeFXRole::PointLight),
            _ => Err(()),
        }
    }
}

impl ShapeFXRole {
    pub fn iterator() -> impl Iterator<Item = ShapeFXRole> {
        [ShapeFXRole::MaterialGeometry, ShapeFXRole::Gradient]
            .iter()
            .copied()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeFX {
    pub id: Uuid,
    pub role: ShapeFXRole,
    pub values: ValueContainer,

    pub position: Vec2<i32>,

    // Used for precomputing values from the Container
    #[serde(skip)]
    precomputed: Vec<Vec4<f32>>,
}

impl ShapeFX {
    pub fn new(role: ShapeFXRole) -> Self {
        let values = ValueContainer::default();

        Self {
            id: Uuid::new_v4(),
            role,
            values,
            position: Vec2::new(20, 20),
            precomputed: vec![],
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            MaterialGeometry => "Geometry".into(),
            Gradient => "Gradient".into(),
            Color => "Color".into(),
            Outline => "Outline".into(),
            NoiseOverlay => "Noise Overlay".into(),
            Glow => "Glow".into(),
            Wood => "Wood".into(),
            LinedefGeometry => "Linedef Geometry".into(),
            SectorGeometry => "Sector Geometry".into(),
            Flatten => "Flatten".into(),
            Render => "Render".into(),
            Fog => "Fog".into(),
            Sky => "Sky".into(),
            Material => "Material".into(),
            PointLight => "Point Light".into(),
        }
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            MaterialGeometry | SectorGeometry | LinedefGeometry => {
                vec![]
            }
            Render => {
                vec![
                    TheNodeTerminal {
                        name: "camera".into(),
                        category_name: "Render".into(),
                    },
                    TheNodeTerminal {
                        name: "fx".into(),
                        category_name: "Render".into(),
                    },
                ]
            }
            Fog | Sky => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "Render".into(),
                }]
            }
            Flatten => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "Modifier".into(),
                }]
            }
            Material | PointLight => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "FX".into(),
                }]
            }
            _ => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "ShapeFX".into(),
                }]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            MaterialGeometry => {
                vec![
                    TheNodeTerminal {
                        name: "inside".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "outside".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            LinedefGeometry => {
                vec![
                    TheNodeTerminal {
                        name: "modifier".into(),
                        category_name: "modifier".into(),
                    },
                    TheNodeTerminal {
                        name: "row1".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "row2".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "row3".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "row4".into(),
                        category_name: "FX".into(),
                    },
                ]
            }
            SectorGeometry => {
                vec![
                    TheNodeTerminal {
                        name: "ground".into(),
                        category_name: "Modifier".into(),
                    },
                    TheNodeTerminal {
                        name: "ceiling".into(),
                        category_name: "Modifier".into(),
                    },
                    TheNodeTerminal {
                        name: "ground".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "ceiling".into(),
                        category_name: "FX".into(),
                    },
                ]
            }
            Render => {
                vec![
                    TheNodeTerminal {
                        name: "hit".into(),
                        category_name: "Render".into(),
                    },
                    TheNodeTerminal {
                        name: "miss".into(),
                        category_name: "Render".into(),
                    },
                ]
            }
            Fog | Sky => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "Render".into(),
                }]
            }
            Flatten => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "Modifier".into(),
                }]
            }
            Material | PointLight => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "FX".into(),
                }]
            }
            _ => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "ShapeFX".into(),
                }]
            }
        }
    }

    /// Modify the given heightmap with the region nodes of the given sector
    pub fn sector_modify_heightmap(
        &self,
        sector: &Sector,
        map: &Map,
        _terrain: &Terrain,
        bbox: &BBox,
        chunk: &TerrainChunk,
        heights: &mut FxHashMap<(i32, i32), f32>,
    ) {
        #[allow(clippy::single_match)]
        match self.role {
            Flatten => {
                let bevel = self.values.get_float_default("bevel", 0.5);
                let floor_height = sector.properties.get_float_default("floor_height", 0.0);

                let mut bounds = sector.bounding_box(map);
                bounds.expand(Vec2::broadcast(bevel));

                let min_x = bounds.min.x.floor() as i32;
                let max_x = bounds.max.x.ceil() as i32;
                let min_y = bounds.min.y.floor() as i32;
                let max_y = bounds.max.y.ceil() as i32;

                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        let p = Vec2::new(x as f32, y as f32);

                        if !bbox.contains(p) {
                            continue;
                        }

                        let Some(sd) = sector.signed_distance(map, p) else {
                            continue;
                        };

                        if sd < bevel {
                            let local = chunk.world_to_local(Vec2::new(x, y));
                            let s = Self::smoothstep(0.0, bevel, bevel - sd);
                            let original =
                                *heights.get(&(local.x, local.y)).unwrap_or(&floor_height);
                            let new_height = original * (1.0 - s) + floor_height * s;
                            heights.insert((local.x, local.y), new_height);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Modify the given heightmap with the region nodes of the given sector
    pub fn linedef_modify_heightmap(
        &self,
        linedefs: &Vec<Linedef>,
        map: &Map,
        _terrain: &Terrain,
        bbox: &BBox,
        chunk: &TerrainChunk,
        heights: &mut FxHashMap<(i32, i32), f32>,
    ) {
        #[allow(clippy::single_match)]
        match self.role {
            ShapeFXRole::Flatten => {
                let bevel = self.values.get_float_default("bevel", 0.5);

                for linedef in linedefs {
                    let Some(start) = map.vertices.iter().find(|v| v.id == linedef.start_vertex)
                    else {
                        continue;
                    };
                    let Some(end) = map.vertices.iter().find(|v| v.id == linedef.end_vertex) else {
                        continue;
                    };

                    let start_pos = start.as_vec2();
                    let end_pos = end.as_vec2();

                    let height_start = start.properties.get_float_default("height", 0.0);
                    let height_end = end.properties.get_float_default("height", 0.0);

                    let dir = (end_pos - start_pos).normalized();
                    let len = (end_pos - start_pos).magnitude();
                    let normal = vek::Vec2::new(-dir.y, dir.x); // perpendicular

                    let steps = (len.ceil() as i32).max(1);

                    for i in 0..=steps {
                        let t = i as f32 / steps as f32;
                        let p = Vec2::lerp(start_pos, end_pos, t);
                        // let s = Self::smoothstep(0.0, 1.0, t);
                        // let p = start_pos.lerp(end_pos, s);
                        let height = height_start * (1.0 - t) + height_end * t;

                        let side_steps = (bevel.ceil() as i32).max(1);
                        for s in -side_steps..=side_steps {
                            let offset = normal * (s as f32 * (bevel / side_steps as f32));
                            let pos = p + offset;

                            if !bbox.contains(pos) {
                                continue;
                            }

                            let world = vek::Vec2::new(pos.x.round(), pos.y.round());
                            let local = chunk
                                .world_to_local(vek::Vec2::new(world.x as i32, world.y as i32));

                            let dist = (offset.magnitude() / bevel).clamp(0.0, 1.0);
                            let blend = Self::smoothstep(0.0, 1.0, 1.0 - dist);

                            let original = *heights.get(&(local.x, local.y)).unwrap_or(&height);
                            let new_height = original * (1.0 - blend) + height * blend;

                            heights.insert((local.x, local.y), new_height);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn render_setup(&mut self, hour: f32) {
        self.precomputed.clear();
        match &self.role {
            Fog => {
                let fog_color = self
                    .values
                    .get_color_default("fog_color", TheColor::black())
                    .to_vec4();

                let end = self.values.get_float_default("fog_end_distance", 30.0);
                let fade = self.values.get_float_default("fog_fade_out", 20.0).max(1.0);

                self.precomputed.push(fog_color);
                self.precomputed.push(Vec4::new(end, fade, 0.0, 0.0));
            }
            Sky => {
                fn smoothstep_transition(hour: f32) -> f32 {
                    let dawn = ((hour - 6.0).clamp(0.0, 2.0) / 2.0).powi(2)
                        * (3.0 - 2.0 * (hour - 6.0).clamp(0.0, 2.0) / 2.0);
                    let dusk = ((20.0 - hour).clamp(0.0, 2.0) / 2.0).powi(2)
                        * (3.0 - 2.0 * (20.0 - hour).clamp(0.0, 2.0) / 2.0);

                    match hour {
                        h if h < 6.0 => 0.0,
                        h if h < 8.0 => dawn,
                        h if h < 18.0 => 1.0,
                        h if h < 20.0 => dusk,
                        _ => 0.0,
                    }
                }

                // Precompute sun position and atmospheric values
                // daylight window
                let sunrise = 6.0;
                let sunset = 20.0;

                let t_day = ((hour - sunrise) / (sunset - sunrise)).clamp(0.0, 1.0);

                let theta = t_day * std::f32::consts::PI;

                let sun_dir = Vec3::new(
                    theta.cos(), // +1 at sunrise, −1 at sunset
                    theta.sin(), //  0 at horizon, +1 overhead
                    0.0,
                );

                // Keep existing day factor calculation
                let day_factor = smoothstep_transition(hour);

                // Store in precomputed[0] as before
                self.precomputed
                    .push(Vec4::new(sun_dir.x, sun_dir.y, sun_dir.z, day_factor));

                // Precompute haze color (rgba)
                let haze_color = Vec4::lerp(
                    Vec4::new(0.1, 0.1, 0.15, 0.0), // Night haze
                    Vec4::new(0.3, 0.3, 0.35, 0.0), // Day haze
                    day_factor,
                );
                self.precomputed.push(haze_color);

                let day_horizon = self
                    .values
                    .get_color_default(
                        "day_horizon",
                        TheColor::from(Vec4::new(0.87, 0.80, 0.70, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(day_horizon);

                let day_zenith = self
                    .values
                    .get_color_default(
                        "day_zenith",
                        TheColor::from(Vec4::new(0.36, 0.62, 0.98, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(day_zenith);

                let night_horizon = self
                    .values
                    .get_color_default(
                        "night_horizon",
                        TheColor::from(Vec4::new(0.03, 0.04, 0.08, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(night_horizon);

                let night_zenith = self
                    .values
                    .get_color_default(
                        "night_zenith",
                        TheColor::from(Vec4::new(0.00, 0.01, 0.05, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(night_zenith);
            }
            _ => {}
        }
    }

    pub fn render_hit_d3(
        &self,
        color: &mut Vec4<f32>,
        camera_pos: &Vec3<f32>,
        world_hit: &Vec3<f32>,
        _normal: &Vec3<f32>,
        _rasterizer: &Rasterizer,
        _time: f32,
    ) {
        #[allow(clippy::single_match)]
        match &self.role {
            Fog => {
                let distance = (world_hit - camera_pos).magnitude();
                let end = self.precomputed[1].x;
                let fade = self.precomputed[1].y;

                if distance > end {
                    let t = ((distance - end) / fade).clamp(0.0, 1.0);
                    *color = *color * (1.0 - t) + self.precomputed[0] * t;
                }
            }
            _ => {}
        }
    }

    pub fn render_ambient_color(&self, _hour: f32) -> Option<Vec4<f32>> {
        #[allow(clippy::single_match)]
        match &self.role {
            Sky => {
                // 0 : sun_dir.xyz  day_factor.w
                // 2 : day_horizon
                // 3 : day_zenith
                // 4 : night_horizon
                // 5 : night_zenith
                let day_factor = self.precomputed[0].w;

                let day_h = self.precomputed[2];
                let day_z = self.precomputed[3];
                let night_h = self.precomputed[4];
                let night_z = self.precomputed[5];

                // quick cosine-weighted average for each half-sphere
                let day_avg = (day_h * 0.5) + (day_z * 0.5);
                let night_avg = (night_h * 0.5) + (night_z * 0.5);

                // Blend between day and night tones by the pre-computed factor
                let c = Vec4::lerp(night_avg, day_avg, day_factor);

                let min_lim = 0.2;

                Some(Vec4::new(
                    linear_to_srgb(c.x.max(min_lim)),
                    linear_to_srgb(c.y.max(min_lim)),
                    linear_to_srgb(c.z.max(min_lim)),
                    1.0,
                ))
            }
            _ => None,
        }
    }

    pub fn render_miss_d3(
        &self,
        color: &mut Vec4<f32>,
        _camera_pos: &Vec3<f32>,
        ray: &Ray,
        _uv: &Vec2<f32>,
        _hour: f32,
    ) {
        #[allow(clippy::single_match)]
        match &self.role {
            Sky => {
                let sun_data = self.precomputed[0];
                let haze_color = self.precomputed[1];

                let sun_dir = Vec3::new(sun_data.x, sun_data.y, sun_data.z);
                let day_factor = sun_data.w;

                let up = ray.dir.y.clamp(-1.0, 1.0);
                let t = (up + 1.0) * 0.5;

                let day_zenith = self.precomputed[3];
                let day_horizon = self.precomputed[2];
                let night_zenith = self.precomputed[5];
                let night_horizon = self.precomputed[4];

                *color = Vec4::lerp(
                    Vec4::lerp(night_horizon, night_zenith, t),
                    Vec4::lerp(day_horizon, day_zenith, t),
                    day_factor,
                );

                // Atmospheric effects
                let haze = (1.0 - up).powi(3);
                let fog = haze_color * haze * 0.3;
                *color = *color * (1.0 - haze * 0.2) + fog;

                // Sun rendering
                if day_factor > 0.0 {
                    const SUN_RADIUS: f32 = 0.04;
                    let dot = ray.dir.dot(sun_dir).clamp(-1.0, 1.0);
                    let dist = (1.0 - dot).max(0.0);

                    if dist < SUN_RADIUS {
                        let k = 1.0 - dist / SUN_RADIUS;
                        let glare = k * k * (3.0 - 2.0 * k); // Smoothstep falloff
                        *color += Vec4::new(1.0, 0.85, 0.6, 0.0) * glare * day_factor;
                    }
                }

                if ray.dir.y > 0.0 {
                    const CLOUD_HEIGHT: f32 = 1500.0;
                    let t_hit = (CLOUD_HEIGHT - _camera_pos.y) / ray.dir.y;

                    if t_hit.is_finite() && t_hit > 0.0 {
                        let hit = *_camera_pos + ray.dir * t_hit;
                        let uv = Vec2::new(hit.x, hit.z) * 0.0005;

                        // let octaves = 1;
                        // let freq_falloff = 0.5;
                        // let lacunarity = 2.0;

                        let mut rng = UniformRandomGen::new(1);
                        let n = perlin_noise_2d(&mut rng, uv.x, uv.y, 5);

                        // let n = fractal_noise_add_2d(
                        //     &mut rng,
                        //     uv.x,
                        //     uv.y,
                        //     perlin_noise_2d,
                        //     octaves,
                        //     freq_falloff,
                        //     lacunarity,
                        //     1,
                        // );

                        let alpha_raw = (n + 1.0) * 0.5;
                        let alpha = alpha_raw * (ray.dir.y * 6.0).clamp(0.0, 1.0);

                        if alpha > 0.0 {
                            // ── Base brightness: never drop below 15 % grey
                            let min_whiteness = 0.15;
                            let whiteness = min_whiteness + (0.6 - min_whiteness) * day_factor; // 0.15 at night → 0.6 day
                            let base_colour = Vec4::lerp(*color, Vec4::one(), whiteness);

                            let sun_lit = (ray.dir.dot(sun_dir)).max(0.0).powf(3.0);
                            let rim = if day_factor > 0.0 {
                                // day: warm rim light
                                Vec4::new(1.0, 0.9, 0.8, 1.0) * sun_lit * 0.4 * day_factor
                            } else {
                                // night: cool moonlight at 20 % strength
                                Vec4::new(0.6, 0.7, 1.0, 1.0) * sun_lit * 0.08
                            };

                            let cloud_colour = base_colour + rim;
                            *color = Vec4::lerp(*color, cloud_colour, alpha);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn compile_light(&self, position: Vec3<f32>) -> Option<CompiledLight> {
        match self.role {
            PointLight => {
                let color = self
                    .values
                    .get_color_default("color", TheColor::white())
                    .to_vec3();
                let strength = self.values.get_float_default("strength", 5.0);
                let range = self.values.get_float_default("range", 10.0);
                let flick = self.values.get_float_default("flicker", 0.0);

                Some(CompiledLight {
                    light_type: LightType::Point,
                    position,
                    color: color.into_array(),
                    intensity: strength,
                    emitting: true,
                    start_distance: 0.0,
                    end_distance: range,
                    flicker: flick,
                    // unused fields:
                    direction: Vec3::unit_y(),
                    cone_angle: 0.0,
                    normal: Vec3::unit_y(),
                    width: 0.0,
                    height: 0.0,
                    from_linedef: false,
                })
            }
            _ => None,
        }
    }

    pub fn evaluate_pixel(
        &self,
        ctx: &ShapeContext,
        color: Option<Vec4<f32>>,
        palette: &ThePalette,
    ) -> Option<Vec4<f32>> {
        match self.role {
            MaterialGeometry => None,
            /*
            Gradient => {
                let alpha = 1.0 - ShapeFX::smoothstep(-ctx.anti_aliasing, 0.0, ctx.distance);
                if alpha > 0.0 {
                    let mut from = Vec4::zero();
                    let top_index = self.values.get_int_default("from", 0);
                    if let Some(Some(top_color)) = palette.colors.get(top_index as usize) {
                        from = top_color.to_vec4();
                    }
                    let mut to = Vec4::zero();
                    let bottom_index = self.values.get_int_default("to", 1);
                    if let Some(Some(bottom_color)) = palette.colors.get(bottom_index as usize) {
                        to = bottom_color.to_vec4();
                    }

                    let angle_rad =
                        (90.0 - self.values.get_float_default("direction", 0.0)).to_radians();
                    let dir = Vec2::new(angle_rad.cos(), angle_rad.sin());

                    let pixel_size = self.values.get_float_default("pixelsize", 0.05);
                    let snapped_uv = Vec2::new(
                        (ctx.uv.x / pixel_size).floor() * pixel_size,
                        (ctx.uv.y / pixel_size).floor() * pixel_size,
                    );

                    let centered_uv = snapped_uv - Vec2::new(0.5, 0.5);
                    let projection = centered_uv.dot(dir);
                    let mut t =
                        (projection / std::f32::consts::FRAC_1_SQRT_2 * 0.5 + 0.5).clamp(0.0, 1.0);
                    if let Some(line_t) = ctx.t {
                        t = line_t.fract();
                    }

                    let dithering = self.values.get_int_default("dithering", 1);
                    if dithering == 1 {
                        let px = (ctx.uv.x / pixel_size).floor() as i32;
                        let py = (ctx.uv.y / pixel_size).floor() as i32;
                        let checker = ((px + py) % 2) as f32 * 0.03; // small tweak value
                        t = (t + checker).clamp(0.0, 1.0);
                    }

                    let mut c = from * (1.0 - t) + to * t;
                    /*
                    c.w = 1.0;
                    if let Some(index) = palette.find_closest_color_index(&TheColor::from(c)) {
                        if let Some(Some(col)) = palette.colors.get(index) {
                            c = col.to_vec4();
                        }
                    }*/
                    c.w = alpha;
                    Some(c)
                } else {
                    None
                }
            }*/
            Gradient => {
                let pixel_size = 0.05;
                let steps = self.values.get_int_default("steps", 4).max(1);
                let blend_mode = self.values.get_int_default("blend_mode", 0);

                let from_index = self.values.get_int_default("edge", 0);
                let to_index = self.values.get_int_default("interior", 1);

                let mut from = palette
                    .colors
                    .get(from_index as usize)
                    .and_then(|c| c.clone())
                    .unwrap_or(TheColor::black())
                    .to_vec4();
                if blend_mode == 1 && color.is_some() {
                    from = color.unwrap();
                }

                let to = palette
                    .colors
                    .get(to_index as usize)
                    .and_then(|c| c.clone())
                    .unwrap_or(TheColor::white())
                    .to_vec4();

                let thickness = self.values.get_float_default("thickness", 40.0);
                let offset = self.values.get_float_default("distance_offset", 0.0);
                let depth = (-(ctx.distance + offset)).clamp(0.0, thickness);

                let snapped_depth = (depth / pixel_size).floor() * pixel_size;
                let mut t = (snapped_depth / thickness).clamp(0.0, 1.0);

                if let Some(line_t) = ctx.t {
                    let line_mode = self.values.get_int_default("line_mode", 0);
                    if line_mode == 1 {
                        let line_factor = line_t.clamp(0.0, 1.0);
                        let radial_factor = (depth / thickness).clamp(0.0, 1.0);
                        t = radial_factor * (1.0 - line_factor);
                    }
                }

                let px = (ctx.uv.x / pixel_size).floor() as i32;
                let py = (ctx.uv.y / pixel_size).floor() as i32;

                let bx = (px & 3) as usize;
                let by = (py & 3) as usize;
                let threshold = BAYER_4X4[by][bx];

                let ft = t * steps as f32;
                let base_step = ft.floor();
                let step_frac = ft - base_step;

                let dithered_step = if step_frac > threshold {
                    base_step + 1.0
                } else {
                    base_step
                }
                .min((steps - 1) as f32);

                let quantized_t = dithered_step / (steps - 1).max(1) as f32;

                let color = from * (1.0 - quantized_t) + to * quantized_t;
                Some(Vec4::new(color.x, color.y, color.z, 1.0))
            }
            Color => {
                let alpha = if ctx.distance > 0.0 {
                    1.0
                } else {
                    1.0 - ShapeFX::smoothstep(-ctx.anti_aliasing, 0.0, ctx.distance)
                };
                if alpha > 0.0 {
                    let mut color = Vec4::zero();
                    let index = self.values.get_int_default("color", 0);
                    if let Some(Some(col)) = palette.colors.get(index as usize) {
                        color = col.to_vec4();
                    }
                    color.w = alpha;
                    Some(color)
                } else {
                    None
                }
            }
            Outline => {
                let mut color = Vec4::zero();
                let index = self.values.get_int_default("color", 0);
                if let Some(Some(col)) = palette.colors.get(index as usize) {
                    color = col.to_vec4();
                }
                let thickness = self.values.get_float_default("thickness", 1.5);
                if ctx.distance < 0.0 && ctx.distance >= -thickness {
                    Some(color)
                } else {
                    None
                }
            }
            NoiseOverlay => {
                let pixel_size = self.values.get_float_default("pixel_size", 0.05);
                let randomness = self.values.get_float_default("randomness", 0.2);
                let octaves = self.values.get_int_default("octaves", 3);

                if let Some(mut color) = color {
                    // Generate noise using UV and pixel snapping
                    let uv = ctx.uv;
                    let scale = Vec2::broadcast(1.0 / pixel_size);
                    let noise_value = self.noise2d(&uv, scale, octaves); // [0.0, 1.0]

                    let n = (noise_value * 2.0 - 1.0) * randomness; // remap to [-1, 1] and scale

                    color.x = (color.x + n).clamp(0.0, 1.0);
                    color.y = (color.y + n).clamp(0.0, 1.0);
                    color.z = (color.z + n).clamp(0.0, 1.0);
                    Some(color)
                } else {
                    None
                }
            }
            Glow => {
                let thickness = self.values.get_float_default("radius", 10.0);
                if ctx.distance > 0.0 && ctx.distance <= thickness {
                    let index = self.values.get_int_default("color", 0);
                    let mut color = palette
                        .colors
                        .get(index as usize)
                        .and_then(|c| c.clone())
                        .unwrap_or(TheColor::white())
                        .to_vec4();

                    let t = (ctx.distance / thickness).clamp(0.0, 1.0);
                    let alpha = 1.0 - ShapeFX::smoothstep(0.0, 1.0, t);
                    color.w = alpha;

                    Some(color)
                } else {
                    None
                }
            }
            ShapeFXRole::Wood => {
                let alpha = 1.0 - ShapeFX::smoothstep(-ctx.anti_aliasing, 0.0, ctx.distance);
                if alpha <= 0.0 {
                    return None;
                }

                let light_idx = self.values.get_int_default("light", 0);
                let dark_idx = self.values.get_int_default("dark", 1);

                let light = palette
                    .colors
                    .get(light_idx as usize)
                    .and_then(|c| c.clone())
                    .unwrap_or(TheColor::white())
                    .to_vec4();

                let dark = palette
                    .colors
                    .get(dark_idx as usize)
                    .and_then(|c| c.clone())
                    .unwrap_or(TheColor::black())
                    .to_vec4();

                let direction_deg = self.values.get_float_default("direction", 0.0);
                let scale = self.values.get_float_default("grain_scale", 4.0); // px between streaks
                let streak_noise = self.values.get_float_default("streak_noise", 1.5); // jaggedness
                let fine_noise = self.values.get_float_default("fine_noise", 0.10); // subtle speckle
                let octaves = self.values.get_int_default("octaves", 3);

                let dir_rad = direction_deg.to_radians();
                let axis = Vec2::new(dir_rad.cos(), dir_rad.sin()); // along plank
                let perpendicular = Vec2::new(-axis.y, axis.x); // across plank

                // Distance “across” the plank controls the stripe colour.
                let across = ctx.uv.dot(perpendicular) * scale; // repeat every 'scale' px
                // Low-freq noise makes the stripes wavy
                let wobble = self.noise2d(&ctx.uv, Vec2::broadcast(0.5), octaves) * streak_noise;
                // Sharpen to make pronounced early/late wood bands
                // let stripe = (across + wobble).fract(); // 0..1 saw wave
                // let stripe_mask = (stripe.min(1.0 - stripe)).powf(0.4);
                // -- 4. main streaks -------------------------------------------------
                let raw = across + wobble;
                let mut s = raw.fract(); // [-1,1)        <-- may be negative
                if s < 0.0 {
                    s += 1.0;
                } // wrap into 0‥1

                // triangle wave: 0 at edges, 1 in the middle
                let stripe_mask = 1.0 - (2.0 * s - 1.0).abs(); // 0‥1
                let stripe_mask = stripe_mask.powf(0.4); // sharpen

                // === 5. fine noise overlay =====================================
                let grain = self.noise2d(&(ctx.uv * 120.0), Vec2::one(), 1) * fine_noise;

                // === 6. final blend ============================================
                let t = (stripe_mask + grain).clamp(0.0, 1.0);
                let mut c = light * (1.0 - t) + dark * t;
                c.w = alpha;
                c = c.map(|v| v.clamp(0.0, 1.0));
                Some(c)
            }
            _ => None,
        }
    }

    /// The parameters for the shapefx
    pub fn params(&self) -> Vec<ShapeFXParam> {
        let mut params = vec![];
        match self.role {
            /*
            Gradient => {
                params.push(ShapeFXParam::Float(
                    "direction".into(),
                    "Direction".into(),
                    "The direction of the gradient.".into(),
                    self.values.get_float_default("direction", 0.0),
                    0.0..=360.0,
                ));
                params.push(ShapeFXParam::Float(
                    "pixelsize".into(),
                    "Pixel Size".into(),
                    "The direction of the gradient.".into(),
                    self.values.get_float_default("pixelsize", 0.05),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Selector(
                    "dithering".into(),
                    "Dithering".into(),
                    "Dithering options for the gradient.".into(),
                    vec!["None".into(), "Checker".into()],
                    self.values.get_int_default("dithering", 1),
                ));
                params.push(ShapeFXParam::PaletteIndex(
                    "from".into(),
                    "From".into(),
                    "The start color of the gradient.".into(),
                    self.values.get_int_default("from", 0),
                ));
                params.push(ShapeFXParam::PaletteIndex(
                    "to".into(),
                    "To".into(),
                    "The end color of the gradient.".into(),
                    self.values.get_int_default("to", 1),
                ))
            }*/
            Gradient => {
                params.push(ShapeFXParam::PaletteIndex(
                    "edge".into(),
                    "Edge Color".into(),
                    "The color at the shape's edge.".into(),
                    self.values.get_int_default("edge", 0),
                ));

                params.push(ShapeFXParam::PaletteIndex(
                    "interior".into(),
                    "Interior Color".into(),
                    "The color towards the shape center.".into(),
                    self.values.get_int_default("interior", 1),
                ));

                params.push(ShapeFXParam::Float(
                    "thickness".into(),
                    "Thickness".into(),
                    "How far the gradient extends inward.".into(),
                    self.values.get_float_default("thickness", 40.0),
                    0.0..=100.0,
                ));
                params.push(ShapeFXParam::Int(
                    "steps".into(),
                    "Steps".into(),
                    "Number of shading bands.".into(),
                    self.values.get_int_default("steps", 4),
                    1..=8,
                ));
                params.push(ShapeFXParam::Selector(
                    "blend_mode".into(),
                    "Blend Mode".into(),
                    "If enabled, uses the incoming color from the previous node as the edge color instead of the palette."
                        .into(),
                    vec!["Off".into(), "Use Incoming Color".into()],
                    self.values.get_int_default("blend_mode", 0),
                ));
                params.push(ShapeFXParam::Selector(
                    "line_mode".into(),
                    "Line Mode".into(),
                    "If the geometry is a line, choose how the gradient is applied: either fading in from the edge (Outside In), or along the line's direction (Line Direction)."
                        .into(),
                    vec!["Outside In".into(), "Line Direction".into()],
                    self.values.get_int_default("line_mode", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "distance_offset".into(),
                    "Distance Offset".into(),
                    "Shift the start of the gradient inward or outward from the shape edge.".into(),
                    self.values.get_float_default("distance_offset", 0.0),
                    -100.0..=100.0,
                ));
            }
            Color => {
                params.push(ShapeFXParam::PaletteIndex(
                    "color".into(),
                    "Color".into(),
                    "The fill color.".into(),
                    self.values.get_int_default("color", 0),
                ));
            }
            Outline => {
                params.push(ShapeFXParam::PaletteIndex(
                    "color".into(),
                    "Color".into(),
                    "The fill color.".into(),
                    self.values.get_int_default("color", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "thickness".into(),
                    "Thickness.".into(),
                    "The thickness of the outlint.".into(),
                    self.values.get_float_default("pixelsize", 1.5),
                    0.0..=10.0,
                ));
            }
            NoiseOverlay => {
                params.push(ShapeFXParam::Float(
                    "pixel_size".into(),
                    "Pixel Size".into(),
                    "Size of the noise pixel grid.".into(),
                    self.values.get_float_default("pixel_size", 0.05),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Float(
                    "randomness".into(),
                    "Randomness".into(),
                    "Randomness factor applied to each pixel.".into(),
                    self.values.get_float_default("randomness", 0.2),
                    0.0..=2.0,
                ));
                params.push(ShapeFXParam::Int(
                    "octaves".into(),
                    "Octaves".into(),
                    "Number of noise layers.".into(),
                    self.values.get_int_default("octaves", 3),
                    0..=6,
                ));
            }
            Glow => {
                params.push(ShapeFXParam::PaletteIndex(
                    "color".into(),
                    "Glow Color".into(),
                    "Color of the glow.".into(),
                    self.values.get_int_default("color", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "radius".into(),
                    "Glow Radius".into(),
                    "How far the glow extends outside the shape.".into(),
                    self.values.get_float_default("radius", 10.0),
                    0.0..=100.0,
                ));
            }
            ShapeFXRole::Wood => {
                params.push(ShapeFXParam::PaletteIndex(
                    "light".into(),
                    "Light Colour".into(),
                    "Pale early-wood streaks.".into(),
                    self.values.get_int_default("light", 0),
                ));
                params.push(ShapeFXParam::PaletteIndex(
                    "dark".into(),
                    "Dark Colour".into(),
                    "Late-wood streaks / grain.".into(),
                    self.values.get_int_default("dark", 1),
                ));
                params.push(ShapeFXParam::Float(
                    "grain_scale".into(),
                    "Streak Spacing".into(),
                    "Average pixel distance between streaks.".into(),
                    self.values.get_float_default("grain_scale", 4.0),
                    0.5..=50.0,
                ));
                params.push(ShapeFXParam::Float(
                    "streak_noise".into(),
                    "Streak Noise".into(),
                    "Side-to-side waviness of the streaks.".into(),
                    self.values.get_float_default("streak_noise", 1.5),
                    0.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "fine_noise".into(),
                    "Fine Grain".into(),
                    "Subtle high-frequency speckles.".into(),
                    self.values.get_float_default("fine_noise", 0.10),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Int(
                    "octaves".into(),
                    "Noise Octaves".into(),
                    "Detail levels for streak wobble.".into(),
                    self.values.get_int_default("octaves", 3),
                    0..=6,
                ));
                params.push(ShapeFXParam::Float(
                    "direction".into(),
                    "Direction".into(),
                    "Plank direction (°).".into(),
                    self.values.get_float_default("direction", 0.0),
                    0.0..=360.0,
                ));
            }
            Flatten => {
                params.push(ShapeFXParam::Float(
                    "bevel".into(),
                    "Bevel".into(),
                    "Smoothly blends the shape's height into the surrounding terrain over this distance.".into(),
                    self.values.get_float_default("bevel", 0.5),
                    0.0..=10.0,
                ));
            }
            ShapeFXRole::Fog => {
                params.push(ShapeFXParam::Color(
                    "fog_color".into(),
                    "Fog Color".into(),
                    "Colour applied to distant fragments.".into(),
                    self.values
                        .get_color_default("fog_color", TheColor::black()),
                ));
                params.push(ShapeFXParam::Float(
                    "fog_end_distance".into(),
                    "End Distance".into(),
                    "World-space distance where fog is 100 % opaque.".into(),
                    self.values.get_float_default("fog_end_distance", 30.0),
                    0.0..=2_000.0,
                ));
                params.push(ShapeFXParam::Float(
                    "fog_fade_out".into(),
                    "Fade-out Length".into(),
                    "How far the fog takes to fade back to clear after the end distance.".into(),
                    self.values.get_float_default("fog_fade_out", 20.0),
                    0.0..=2_000.0,
                ));
            }
            ShapeFXRole::Sky => {
                params.push(ShapeFXParam::Color(
                    "day_horizon".into(),
                    "Day Horizon".into(),
                    "Colour blended along the horizon during daylight.".into(),
                    self.values
                        .get_color_default("day_horizon", TheColor::new(0.87, 0.80, 0.70, 1.0)),
                ));
                params.push(ShapeFXParam::Color(
                    "day_zenith".into(),
                    "Day Zenith".into(),
                    "Colour blended straight overhead during daylight.".into(),
                    self.values
                        .get_color_default("day_zenith", TheColor::new(0.36, 0.62, 0.98, 1.0)),
                ));
                params.push(ShapeFXParam::Color(
                    "night_horizon".into(),
                    "Night Horizon".into(),
                    "Colour along the horizon after sunset / before sunrise.".into(),
                    self.values
                        .get_color_default("night_horizon", TheColor::new(0.03, 0.04, 0.08, 1.0)),
                ));
                params.push(ShapeFXParam::Color(
                    "night_zenith".into(),
                    "Night Zenith".into(),
                    "Colour straight overhead during the night.".into(),
                    self.values
                        .get_color_default("night_zenith", TheColor::new(0.00, 0.01, 0.05, 1.0)),
                ));
            }
            Material => {
                params.push(ShapeFXParam::Selector(
                    "role".into(),
                    "Type".into(),
                    "The material type.".into(),
                    vec![
                        "Matte".into(),
                        "Glossy".into(),
                        "Mettalic".into(),
                        "Transparent".into(),
                        "Emissive".into(),
                    ],
                    self.values.get_int_default("role", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "value".into(),
                    "Value".into(),
                    "The material value.".into(),
                    self.values.get_float_default("value", 1.0),
                    0.0..=1.0,
                ));
            }
            ShapeFXRole::PointLight => {
                params.push(ShapeFXParam::Color(
                    "color".into(),
                    "Colour".into(),
                    "Light colour".into(),
                    self.values.get_color_default("color", TheColor::white()),
                ));
                params.push(ShapeFXParam::Float(
                    "strength".into(),
                    "Strength".into(),
                    "How bright the light is.".into(),
                    self.values.get_float_default("strength", 5.0),
                    0.0..=100.0,
                ));
                params.push(ShapeFXParam::Float(
                    "range".into(),
                    "Range".into(),
                    "Rough maximum reach.".into(),
                    self.values.get_float_default("range", 10.0),
                    0.5..=100.0,
                ));
                params.push(ShapeFXParam::Float(
                    "flicker".into(),
                    "Flicker".into(),
                    "0 = steady, 1 = candles.".into(),
                    self.values.get_float_default("flicker", 0.0),
                    0.0..=1.0,
                ));
            }
            _ => {}
        }
        params
    }

    #[inline]
    fn _lerp(a: f32, b: f32, t: f32) -> f32 {
        a * (1.0 - t) + b * t
    }

    #[inline]
    pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn noise2d(&self, p: &Vec2<f32>, scale: Vec2<f32>, octaves: i32) -> f32 {
        fn hash(p: Vec2<f32>) -> f32 {
            let mut p3 = Vec3::new(p.x, p.y, p.x).map(|v| (v * 0.13).fract());
            p3 += p3.dot(Vec3::new(p3.y, p3.z, p3.x) + 3.333);
            ((p3.x + p3.y) * p3.z).fract()
        }

        fn noise(x: Vec2<f32>) -> f32 {
            let i = x.map(|v| v.floor());
            let f = x.map(|v| v.fract());

            let a = hash(i);
            let b = hash(i + Vec2::new(1.0, 0.0));
            let c = hash(i + Vec2::new(0.0, 1.0));
            let d = hash(i + Vec2::new(1.0, 1.0));

            let u = f * f * f.map(|v| 3.0 - 2.0 * v);
            f32::lerp(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y
        }

        let mut x = *p * 8.0 * scale;

        if octaves == 0 {
            return noise(x);
        }

        let mut v = 0.0;
        let mut a = 0.5;
        let shift = Vec2::new(100.0, 100.0);
        let rot = Mat2::new(0.5f32.cos(), 0.5f32.sin(), -0.5f32.sin(), 0.5f32.cos());
        for _ in 0..octaves {
            v += a * noise(x);
            x = rot * x * 2.0 + shift;
            a *= 0.5;
        }
        v
    }

    /// Get the dominant node color for sector previews
    pub fn get_dominant_color(&self, palette: &ThePalette) -> Pixel {
        match self.role {
            Gradient => self.get_palette_color("interior", palette),
            _ => self.get_palette_color("color", palette),
        }
    }

    /// Get the color of a given name from the values.
    pub fn get_palette_color(&self, named: &str, palette: &ThePalette) -> Pixel {
        let mut color = BLACK;
        let index = self.values.get_int_default(named, 0);
        if let Some(Some(col)) = palette.colors.get(index as usize) {
            color = col.to_u8_array();
        }
        color
    }
}
