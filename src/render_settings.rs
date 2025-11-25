use scenevm::{Atom, SceneVM};
use vek::Vec4;

/// PBR Render Settings for scenes
/// Corresponds to the uniform parameters (gp0-gp9) in the SceneVM PBR shader
#[derive(Debug, Clone)]
pub struct RenderSettings {
    /// Sky color (RGB) - set from TOML or dynamically by apply_hour()
    pub sky_color: [f32; 3],

    /// Sun color (RGB) - set from TOML or dynamically by apply_hour()
    pub sun_color: [f32; 3],

    /// Sun intensity (brightness multiplier)
    pub sun_intensity: f32,

    /// Sun direction (normalized vector) - set from TOML or dynamically by apply_hour()
    pub sun_direction: [f32; 3],

    /// Sun enabled
    pub sun_enabled: bool,

    /// Ambient color (RGB)
    pub ambient_color: [f32; 3],

    /// Ambient strength (0.0 to 1.0)
    pub ambient_strength: f32,

    /// Fog color (RGB)
    pub fog_color: [f32; 3],

    /// Fog density (0.0 = no fog, higher = denser)
    pub fog_density: f32,

    /// AO samples (number of rays)
    pub ao_samples: f32,

    /// AO radius
    pub ao_radius: f32,

    /// Bump strength (0.0-1.0)
    pub bump_strength: f32,

    /// Max transparency bounces
    pub max_transparency_bounces: f32,

    /// Max shadow distance
    pub max_shadow_distance: f32,

    /// Max sky distance
    pub max_sky_distance: f32,

    /// Max shadow steps (for transparent shadows)
    pub max_shadow_steps: f32,

    /// Daylight simulation settings
    pub simulation: DaylightSimulation,
}

/// Daylight simulation settings for time-of-day rendering
#[derive(Debug, Clone)]
pub struct DaylightSimulation {
    /// Enable procedural daylight simulation
    pub enabled: bool,

    /// Sky color at night
    pub night_sky_color: [f32; 3],

    /// Sky color at sunrise/sunset
    pub morning_sky_color: [f32; 3],

    /// Sky color at midday
    pub midday_sky_color: [f32; 3],

    /// Sky color in the evening
    pub evening_sky_color: [f32; 3],

    /// Sun color at night (moon light)
    pub night_sun_color: [f32; 3],

    /// Sun color at sunrise/sunset
    pub morning_sun_color: [f32; 3],

    /// Sun color at midday
    pub midday_sun_color: [f32; 3],

    /// Sun color in the evening
    pub evening_sun_color: [f32; 3],

    /// Sunrise time (0.0 - 24.0, e.g., 6.5 = 6:30 AM)
    pub sunrise_time: f32,

    /// Sunset time (0.0 - 24.0, e.g., 18.5 = 6:30 PM)
    pub sunset_time: f32,
}

impl Default for DaylightSimulation {
    fn default() -> Self {
        Self {
            enabled: false,
            night_sky_color: [0.02, 0.02, 0.05], // Very dark blue
            morning_sky_color: [1.0, 0.6, 0.4],  // Warm orange morning
            midday_sky_color: [0.529, 0.808, 0.922], // Clear blue
            evening_sky_color: [1.0, 0.5, 0.3],  // Warm orange evening
            night_sun_color: [0.1, 0.1, 0.15],   // Very dim bluish (moon)
            morning_sun_color: [1.0, 0.8, 0.6],  // Warm morning sun
            midday_sun_color: [1.0, 1.0, 0.95],  // Bright white sun
            evening_sun_color: [1.0, 0.7, 0.5],  // Warm evening sun
            sunrise_time: 6.0,
            sunset_time: 18.0,
        }
    }
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            sky_color: [0.529, 0.808, 0.922], // #87CEEB
            sun_color: [1.0, 0.980, 0.804],   // #FFFACD
            sun_intensity: 1.0,
            sun_direction: [-0.5, -1.0, -0.3],
            sun_enabled: true,
            ambient_color: [0.8, 0.8, 0.8],
            ambient_strength: 0.3,
            fog_color: [0.502, 0.502, 0.502], // #808080
            fog_density: 0.0,
            ao_samples: 8.0,
            ao_radius: 0.5,
            bump_strength: 1.0,
            max_transparency_bounces: 8.0,
            max_shadow_distance: 10.0,
            max_sky_distance: 50.0,
            max_shadow_steps: 2.0,
            simulation: DaylightSimulation::default(),
        }
    }
}

impl RenderSettings {
    /// Parse render settings from a TOML string's [render] and [simulation] sections
    pub fn read(&mut self, toml_content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let parsed: toml::Value = toml::from_str(toml_content)?;

        // Parse [render] section
        if let Some(section) = parsed.get("render") {
            if let Some(sky) = section.get("sky_color") {
                self.sky_color =
                    parse_hex_color(sky.as_str().ok_or("sky_color must be a string")?)?;
            }

            if let Some(sun) = section.get("sun_color") {
                self.sun_color =
                    parse_hex_color(sun.as_str().ok_or("sun_color must be a string")?)?;
            }

            if let Some(intensity) = section.get("sun_intensity") {
                self.sun_intensity = intensity
                    .as_float()
                    .ok_or("sun_intensity must be a number")?
                    as f32;
            }

            if let Some(dir) = section.get("sun_direction") {
                let arr = dir.as_array().ok_or("sun_direction must be an array")?;
                if arr.len() != 3 {
                    return Err("sun_direction must have 3 elements".into());
                }
                self.sun_direction = [
                    arr[0]
                        .as_float()
                        .ok_or("sun_direction[0] must be a number")? as f32,
                    arr[1]
                        .as_float()
                        .ok_or("sun_direction[1] must be a number")? as f32,
                    arr[2]
                        .as_float()
                        .ok_or("sun_direction[2] must be a number")? as f32,
                ];
            }

            if let Some(enabled) = section.get("sun_enabled") {
                self.sun_enabled = enabled.as_bool().ok_or("sun_enabled must be a boolean")?;
            }

            if let Some(ambient) = section.get("ambient_color") {
                self.ambient_color =
                    parse_hex_color(ambient.as_str().ok_or("ambient_color must be a string")?)?;
            }

            if let Some(strength) = section.get("ambient_strength") {
                self.ambient_strength = strength
                    .as_float()
                    .ok_or("ambient_strength must be a number")?
                    as f32;
            }

            if let Some(fog) = section.get("fog_color") {
                self.fog_color =
                    parse_hex_color(fog.as_str().ok_or("fog_color must be a string")?)?;
            }

            if let Some(density) = section.get("fog_density") {
                self.fog_density =
                    density.as_float().ok_or("fog_density must be a number")? as f32 / 100.0;
            }

            if let Some(samples) = section.get("ao_samples") {
                self.ao_samples = samples.as_float().ok_or("ao_samples must be a number")? as f32;
            }

            if let Some(radius) = section.get("ao_radius") {
                self.ao_radius = radius.as_float().ok_or("ao_radius must be a number")? as f32;
            }

            if let Some(strength) = section.get("bump_strength") {
                self.bump_strength = strength
                    .as_float()
                    .ok_or("bump_strength must be a number")?
                    as f32;
            }

            if let Some(bounces) = section.get("max_transparency_bounces") {
                self.max_transparency_bounces = bounces
                    .as_float()
                    .ok_or("max_transparency_bounces must be a number")?
                    as f32;
            }

            if let Some(dist) = section.get("max_shadow_distance") {
                self.max_shadow_distance =
                    dist.as_float()
                        .ok_or("max_shadow_distance must be a number")? as f32;
            }

            if let Some(dist) = section.get("max_sky_distance") {
                self.max_sky_distance =
                    dist.as_float().ok_or("max_sky_distance must be a number")? as f32;
            }

            if let Some(steps) = section.get("max_shadow_steps") {
                self.max_shadow_steps = steps
                    .as_float()
                    .ok_or("max_shadow_steps must be a number")?
                    as f32;
            }
        }

        // Parse [simulation] section
        if let Some(section) = parsed.get("simulation") {
            if let Some(enabled) = section.get("enabled") {
                self.simulation.enabled = enabled
                    .as_bool()
                    .ok_or("simulation.enabled must be a boolean")?;
            }

            if let Some(sky) = section.get("night_sky_color") {
                self.simulation.night_sky_color =
                    parse_hex_color(sky.as_str().ok_or("night_sky_color must be a string")?)?;
            }

            if let Some(sky) = section.get("morning_sky_color") {
                self.simulation.morning_sky_color =
                    parse_hex_color(sky.as_str().ok_or("morning_sky_color must be a string")?)?;
            }

            if let Some(sky) = section.get("midday_sky_color") {
                self.simulation.midday_sky_color =
                    parse_hex_color(sky.as_str().ok_or("midday_sky_color must be a string")?)?;
            }

            if let Some(sky) = section.get("evening_sky_color") {
                self.simulation.evening_sky_color =
                    parse_hex_color(sky.as_str().ok_or("evening_sky_color must be a string")?)?;
            }

            if let Some(sun) = section.get("night_sun_color") {
                self.simulation.night_sun_color =
                    parse_hex_color(sun.as_str().ok_or("night_sun_color must be a string")?)?;
            }

            if let Some(sun) = section.get("morning_sun_color") {
                self.simulation.morning_sun_color =
                    parse_hex_color(sun.as_str().ok_or("morning_sun_color must be a string")?)?;
            }

            if let Some(sun) = section.get("midday_sun_color") {
                self.simulation.midday_sun_color =
                    parse_hex_color(sun.as_str().ok_or("midday_sun_color must be a string")?)?;
            }

            if let Some(sun) = section.get("evening_sun_color") {
                self.simulation.evening_sun_color =
                    parse_hex_color(sun.as_str().ok_or("evening_sun_color must be a string")?)?;
            }

            if let Some(time) = section.get("sunrise_time") {
                self.simulation.sunrise_time =
                    time.as_float().ok_or("sunrise_time must be a number")? as f32;
            }

            if let Some(time) = section.get("sunset_time") {
                self.simulation.sunset_time =
                    time.as_float().ok_or("sunset_time must be a number")? as f32;
            }
        }

        Ok(())
    }

    /// Apply time-of-day settings based on the current hour (0.0 - 24.0)
    /// This interpolates sky color, sun color, and calculates sun direction procedurally
    /// Only applies if simulation is enabled
    pub fn apply_hour(&mut self, hour: f32) {
        if !self.simulation.enabled {
            return;
        }

        let sim = &self.simulation;
        let hour = hour % 24.0;

        // Calculate sun position (angle from horizon)
        // Sun rises at sunrise_time, peaks at midday, sets at sunset_time
        let midday = (sim.sunrise_time + sim.sunset_time) / 2.0;

        // Twilight duration (time to fade from/to night)
        let twilight_duration = 1.0; // 1 hour fade
        let evening_end = sim.sunset_time + twilight_duration;
        let morning_start = sim.sunrise_time - twilight_duration;

        let (sky_color, sun_color, sun_angle) = if hour >= morning_start && hour < sim.sunrise_time
        {
            // Pre-sunrise twilight (night fading to morning)
            let t = (hour - morning_start) / twilight_duration;
            let sky = lerp_color(sim.night_sky_color, sim.morning_sky_color, t);
            let sun = lerp_color(sim.night_sun_color, sim.morning_sun_color, t);
            let angle = lerp(-30.0, 0.0, t);
            (sky, sun, angle)
        } else if hour >= sim.sunrise_time && hour < midday {
            // Morning (sunrise to midday)
            let t = (hour - sim.sunrise_time) / (midday - sim.sunrise_time);
            let sky = lerp_color(sim.morning_sky_color, sim.midday_sky_color, t);
            let sun = lerp_color(sim.morning_sun_color, sim.midday_sun_color, t);
            let angle = lerp(0.0, 90.0, t);
            (sky, sun, angle)
        } else if hour >= midday && hour < sim.sunset_time {
            // Afternoon (midday to sunset)
            let t = (hour - midday) / (sim.sunset_time - midday);
            let sky = lerp_color(sim.midday_sky_color, sim.evening_sky_color, t);
            let sun = lerp_color(sim.midday_sun_color, sim.evening_sun_color, t);
            let angle = lerp(90.0, 0.0, t);
            (sky, sun, angle)
        } else if hour >= sim.sunset_time && hour < evening_end {
            // Post-sunset twilight (evening fading to night)
            let t = (hour - sim.sunset_time) / twilight_duration;
            let sky = lerp_color(sim.evening_sky_color, sim.night_sky_color, t);
            let sun = lerp_color(sim.evening_sun_color, sim.night_sun_color, t);
            let angle = lerp(0.0, -30.0, t);
            (sky, sun, angle)
        } else {
            // Deep night - full darkness (before morning_start or after evening_end)
            (sim.night_sky_color, sim.night_sun_color, -30.0)
        };

        // Apply interpolated colors
        self.sky_color = sky_color;
        self.sun_color = sun_color;

        // Calculate sun direction procedurally from time of day
        // Sun travels from east (-1, 0, 0) through zenith (0, -1, 0) to west (1, 0, 0)
        let angle_rad = sun_angle.to_radians();
        let progress = (hour - sim.sunrise_time) / (sim.sunset_time - sim.sunrise_time);
        let progress = progress.clamp(0.0, 1.0);

        // X goes from -1 (east) to 1 (west)
        let x = lerp(-1.0, 1.0, progress);
        // Y is based on angle above horizon
        let y = -angle_rad.sin();
        // Z stays slightly forward
        let z = -0.3;

        self.sun_direction = [x, y, z];
    }

    /// Apply these render settings to a SceneVM instance
    pub fn apply_2d(&self, vm: &mut SceneVM) {
        // gp1: Sky color (RGB) + unused w
        vm.execute(Atom::SetGP1(Vec4::new(
            self.sky_color[0],
            self.sky_color[1],
            self.sky_color[2],
            0.0,
        )));
    }

    /// Apply these render settings to a SceneVM instance
    pub fn apply_3d(&self, vm: &mut SceneVM) {
        // Convert sRGB colors to linear space (gamma 2.2) on CPU instead of per-pixel in shader
        let to_linear = |c: f32| c.powf(2.2);

        // gp0: Sky color (RGB, linear) + unused w
        vm.execute(Atom::SetGP0(Vec4::new(
            to_linear(self.sky_color[0]),
            to_linear(self.sky_color[1]),
            to_linear(self.sky_color[2]),
            0.0,
        )));

        // gp1: Sun color (RGB, linear) + sun intensity (w)
        vm.execute(Atom::SetGP1(Vec4::new(
            to_linear(self.sun_color[0]),
            to_linear(self.sun_color[1]),
            to_linear(self.sun_color[2]),
            self.sun_intensity,
        )));

        // gp2: Sun direction (XYZ, normalized) + sun enabled (w)
        let sun_dir = vek::Vec3::from(self.sun_direction).normalized();
        vm.execute(Atom::SetGP2(Vec4::new(
            sun_dir.x,
            sun_dir.y,
            sun_dir.z,
            if self.sun_enabled { 1.0 } else { 0.0 },
        )));

        // gp3: Ambient color (RGB, linear) + ambient strength (w)
        vm.execute(Atom::SetGP3(Vec4::new(
            to_linear(self.ambient_color[0]),
            to_linear(self.ambient_color[1]),
            to_linear(self.ambient_color[2]),
            self.ambient_strength,
        )));

        // gp4: Fog color (RGB, linear) + fog density (w)
        vm.execute(Atom::SetGP4(Vec4::new(
            to_linear(self.fog_color[0]),
            to_linear(self.fog_color[1]),
            to_linear(self.fog_color[2]),
            self.fog_density,
        )));

        // gp5: Rendering quality settings
        // x: AO samples, y: AO radius, z: Bump strength, w: Max transparency bounces
        vm.execute(Atom::SetGP5(Vec4::new(
            self.ao_samples,
            self.ao_radius,
            self.bump_strength,
            self.max_transparency_bounces,
        )));

        // gp6: Distance settings
        // x: Max shadow distance, y: Max sky distance, z: Max shadow steps, w: unused
        vm.execute(Atom::SetGP6(Vec4::new(
            self.max_shadow_distance,
            self.max_sky_distance,
            self.max_shadow_steps,
            0.0,
        )));
    }
}

/// Linear interpolation between two f32 values
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linear interpolation between two RGB colors
fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

/// Parse a hex color string like "#RRGGBB" or "RRGGBB" into RGB floats (0.0-1.0)
fn parse_hex_color(hex: &str) -> Result<[f32; 3], Box<dyn std::error::Error>> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Err(format!(
            "Invalid hex color: expected 6 characters, got {}",
            hex.len()
        )
        .into());
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}
