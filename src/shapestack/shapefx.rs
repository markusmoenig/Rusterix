use crate::{BLACK, Pixel, ShapeContext, ValueContainer};
use std::fmt;
use std::str::FromStr;
use theframework::prelude::*;
use uuid::Uuid;
use vek::Vec4;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShapeFXRole {
    Geometry,
    Gradient,
    Color,
    Outline,
    NoiseOverlay,
    Glow,
}

impl fmt::Display for ShapeFXRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ShapeFXRole::Geometry => "Geometry",
            ShapeFXRole::Gradient => "Gradient",
            ShapeFXRole::Color => "Color",
            ShapeFXRole::Outline => "Outline",
            ShapeFXRole::NoiseOverlay => "Noise Overlay",
            ShapeFXRole::Glow => "Glow",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ShapeFXRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Geometry" => Ok(ShapeFXRole::Geometry),
            "Gradient" => Ok(ShapeFXRole::Gradient),
            "Color" => Ok(ShapeFXRole::Color),
            "Outline" => Ok(ShapeFXRole::Outline),
            "Noise Overlay" => Ok(ShapeFXRole::NoiseOverlay),
            "Glow" => Ok(ShapeFXRole::Glow),
            _ => Err(()),
        }
    }
}

impl ShapeFXRole {
    pub fn iterator() -> impl Iterator<Item = ShapeFXRole> {
        [ShapeFXRole::Geometry, ShapeFXRole::Gradient]
            .iter()
            .copied()
    }
}

use ShapeFXRole::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeFX {
    pub id: Uuid,
    pub role: ShapeFXRole,
    pub values: ValueContainer,

    pub position: Vec2<i32>,
}

impl ShapeFX {
    pub fn new(role: ShapeFXRole) -> Self {
        let values = ValueContainer::default();

        Self {
            id: Uuid::new_v4(),
            role,
            values,
            position: Vec2::new(20, 20),
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            Geometry => "Geometry".into(),
            Gradient => "Gradient".into(),
            Color => "Color".into(),
            Outline => "Outline".into(),
            NoiseOverlay => "Noise Overlay".into(),
            Glow => "Glow".into(),
        }
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                vec![]
            }
            _ => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                vec![
                    TheNodeTerminal {
                        name: "inside".into(),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: "outside".into(),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            _ => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            } // _ => vec![],
        }
    }

    pub fn evaluate(
        &self,
        ctx: &ShapeContext,
        color: Option<Vec4<f32>>,
        palette: &ThePalette,
    ) -> Option<Vec4<f32>> {
        match self.role {
            Geometry => None,
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
            _ => {}
        }
        params
    }

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
