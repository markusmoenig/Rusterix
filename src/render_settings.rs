use scenevm::{Atom, SceneVM};
use vek::Vec4;

/// PBR Render Settings for scenes
/// Corresponds to the uniform parameters (gp0-gp9) in the SceneVM PBR shader
#[derive(Debug, Clone)]
pub struct RenderSettings {
    /// Sky color (RGB)
    pub sky_color: [f32; 3],

    /// Sun color (RGB)
    pub sun_color: [f32; 3],

    /// Sun intensity (brightness multiplier)
    pub sun_intensity: f32,

    /// Sun direction (normalized vector)
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
        }
    }
}

impl RenderSettings {
    /// Parse render settings from a TOML string's [render] section
    pub fn read(&mut self, toml_content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let parsed: toml::Value = toml::from_str(toml_content)?;

        let section = parsed
            .get("render")
            .ok_or("Missing [render] section in TOML")?;

        // Parse each field if present
        if let Some(sky) = section.get("sky_color") {
            self.sky_color = parse_hex_color(sky.as_str().ok_or("sky_color must be a string")?)?;
        }

        if let Some(sun) = section.get("sun_color") {
            self.sun_color = parse_hex_color(sun.as_str().ok_or("sun_color must be a string")?)?;
        }

        if let Some(intensity) = section.get("sun_intensity") {
            self.sun_intensity = intensity
                .as_float()
                .ok_or("sun_intensity must be a number")? as f32;
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
            self.fog_color = parse_hex_color(fog.as_str().ok_or("fog_color must be a string")?)?;
        }

        if let Some(density) = section.get("fog_density") {
            self.fog_density =
                density.as_float().ok_or("fog_density must be a number")? as f32 / 100.0;
        }

        Ok(())
    }

    /// Apply these render settings to a SceneVM instance
    pub fn apply(&self, vm: &mut SceneVM) {
        // gp0: Sky color (RGB) + unused w
        vm.execute(Atom::SetGP0(Vec4::new(
            self.sky_color[0],
            self.sky_color[1],
            self.sky_color[2],
            0.0,
        )));

        // gp1: Sun color (RGB) + sun intensity (w)
        vm.execute(Atom::SetGP1(Vec4::new(
            self.sun_color[0],
            self.sun_color[1],
            self.sun_color[2],
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

        // gp3: Ambient color (RGB) + ambient strength (w)
        vm.execute(Atom::SetGP3(Vec4::new(
            self.ambient_color[0],
            self.ambient_color[1],
            self.ambient_color[2],
            self.ambient_strength,
        )));

        // gp4: Fog color (RGB) + fog density (w)
        vm.execute(Atom::SetGP4(Vec4::new(
            self.fog_color[0],
            self.fog_color[1],
            self.fog_color[2],
            self.fog_density,
        )));
    }
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
