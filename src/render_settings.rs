use crate::value::ValueContainer;
use crate::value_toml::ValueTomlLoader;
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

    /// Reflection samples (0 = disabled, higher = better quality)
    pub reflection_samples: f32,

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
            reflection_samples: 0.0,
            simulation: DaylightSimulation::default(),
        }
    }
}

impl RenderSettings {
    /// Parse render settings from a TOML string's [render] and [simulation] sections
    pub fn read(&mut self, toml_content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let groups = ValueTomlLoader::from_str(toml_content)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        if let Some(render) = groups.get("render") {
            self.apply_render_values(render)?;
        }

        if let Some(sim) = groups.get("simulation") {
            self.apply_simulation_values(sim)?;
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
        // x: Max shadow distance, y: Max sky distance, z: Max shadow steps, w: Reflection samples
        vm.execute(Atom::SetGP6(Vec4::new(
            self.max_shadow_distance,
            self.max_sky_distance,
            self.max_shadow_steps,
            self.reflection_samples,
        )));
    }
}

impl RenderSettings {
    fn apply_render_values(
        &mut self,
        render: &ValueContainer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(v) = render.get_str("sky_color") {
            self.sky_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("sky_color") {
            self.sky_color = v;
        }

        if let Some(v) = render.get_str("sun_color") {
            self.sun_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("sun_color") {
            self.sun_color = v;
        }

        self.sun_intensity = render.get_float_default("sun_intensity", self.sun_intensity);
        self.sun_direction = render.get_vec3_default("sun_direction", self.sun_direction);
        self.sun_enabled = render.get_bool_default("sun_enabled", self.sun_enabled);

        if let Some(v) = render.get_str("ambient_color") {
            self.ambient_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("ambient_color") {
            self.ambient_color = v;
        }

        self.ambient_strength = render.get_float_default("ambient_strength", self.ambient_strength);

        if let Some(v) = render.get_str("fog_color") {
            self.fog_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("fog_color") {
            self.fog_color = v;
        }

        // keep legacy percent scaling
        if let Some(d) = render.get_float("fog_density") {
            self.fog_density = d / 100.0;
        }
        self.ao_samples = render.get_float_default("ao_samples", self.ao_samples);
        self.ao_radius = render.get_float_default("ao_radius", self.ao_radius);
        self.bump_strength = render.get_float_default("bump_strength", self.bump_strength);
        self.max_transparency_bounces =
            render.get_float_default("max_transparency_bounces", self.max_transparency_bounces);
        self.max_shadow_distance =
            render.get_float_default("max_shadow_distance", self.max_shadow_distance);
        self.max_sky_distance = render.get_float_default("max_sky_distance", self.max_sky_distance);
        self.max_shadow_steps = render.get_float_default("max_shadow_steps", self.max_shadow_steps);
        self.reflection_samples =
            render.get_float_default("reflection_samples", self.reflection_samples);

        Ok(())
    }

    fn apply_simulation_values(
        &mut self,
        sim: &ValueContainer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.simulation.enabled = sim.get_bool_default("enabled", self.simulation.enabled);

        if let Some(v) = sim.get_str("night_sky_color") {
            self.simulation.night_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("night_sky_color") {
            self.simulation.night_sky_color = v;
        }

        if let Some(v) = sim.get_str("morning_sky_color") {
            self.simulation.morning_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("morning_sky_color") {
            self.simulation.morning_sky_color = v;
        }

        if let Some(v) = sim.get_str("midday_sky_color") {
            self.simulation.midday_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("midday_sky_color") {
            self.simulation.midday_sky_color = v;
        }

        if let Some(v) = sim.get_str("evening_sky_color") {
            self.simulation.evening_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("evening_sky_color") {
            self.simulation.evening_sky_color = v;
        }

        if let Some(v) = sim.get_str("night_sun_color") {
            self.simulation.night_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("night_sun_color") {
            self.simulation.night_sun_color = v;
        }

        if let Some(v) = sim.get_str("morning_sun_color") {
            self.simulation.morning_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("morning_sun_color") {
            self.simulation.morning_sun_color = v;
        }

        if let Some(v) = sim.get_str("midday_sun_color") {
            self.simulation.midday_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("midday_sun_color") {
            self.simulation.midday_sun_color = v;
        }

        if let Some(v) = sim.get_str("evening_sun_color") {
            self.simulation.evening_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("evening_sun_color") {
            self.simulation.evening_sun_color = v;
        }

        self.simulation.sunrise_time =
            sim.get_float_default("sunrise_time", self.simulation.sunrise_time);
        self.simulation.sunset_time =
            sim.get_float_default("sunset_time", self.simulation.sunset_time);

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_example_toml() {
        let example = include_str!("../render_settings_example.toml");
        let mut settings = RenderSettings::default();
        settings.read(example).expect("render settings parse");

        assert_eq!(settings.sky_color, [0.5294118, 0.80784315, 0.92156863]); // #87CEEB
        assert_eq!(settings.sun_color, [1.0, 0.98039216, 0.8039216]); // #FFFACD
        assert_eq!(settings.sun_intensity, 1.0);
        assert_eq!(settings.sun_direction, [-0.5, -1.0, -0.3]);
        assert!(settings.sun_enabled);
        assert!(settings.simulation.enabled);
        assert_eq!(settings.simulation.sunrise_time, 6.0);
        assert_eq!(settings.simulation.sunset_time, 18.0);
    }
}
