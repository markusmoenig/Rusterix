use vek::{Vec2, Vec3};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Light {
    PointLight {
        position: Vec3<f32>,
        color: [f32; 3],
        intensity: f32,
        start_distance: f32,
        end_distance: f32,
        flicker: Option<Flicker>, // Optional flickering
    },
    AmbientLight {
        position: Vec3<f32>,
        color: [f32; 3],
        intensity: f32,
    },
    Spotlight {
        position: Vec3<f32>,
        direction: Vec3<f32>,
        color: [f32; 3],
        intensity: f32,
        start_distance: f32,
        end_distance: f32,
        cone_angle: f32,
        flicker: Option<Flicker>, // Optional flickering
    },
    AreaLight {
        position: Vec3<f32>, // Center of the light
        normal: Vec3<f32>,   // Normal vector of the emitting surface
        width: f32,          // Width of the rectangular light
        height: f32,         // Height of the rectangular light
        color: [f32; 3],     // RGB color of the light
        intensity: f32,      // Overall intensity
    },
}

/// Parameters for flickering
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Flicker {
    pub frequency: f32, // How fast the flicker changes (in Hz)
    pub amplitude: f32, // Max intensity change (e.g., 0.2 for 20% flicker)
}

impl Light {
    /// Returns the position of the light, if applicable
    pub fn position(&self) -> Vec3<f32> {
        match *self {
            Light::PointLight { position, .. } => position,
            Light::Spotlight { position, .. } => position,
            Light::AreaLight { position, .. } => position,
            Light::AmbientLight { position, .. } => position,
        }
    }

    pub fn position_2d(&self) -> Vec2<f32> {
        match *self {
            Light::PointLight { position, .. } => Vec2::new(position.x, position.z),
            Light::Spotlight { position, .. } => Vec2::new(position.x, position.z),
            Light::AreaLight { position, .. } => Vec2::new(position.x, position.z),
            Light::AmbientLight { position, .. } => Vec2::new(position.x, position.z),
        }
    }

    /// Calculate the lights intensity and color at a given point
    pub fn color_at(&self, point: Vec3<f32>, time: f32) -> [f32; 3] {
        match *self {
            Light::PointLight {
                position,
                color,
                intensity,
                start_distance,
                end_distance,
                flicker,
            } => {
                let distance = (point - position).magnitude();

                if distance <= start_distance {
                    return apply_flicker(color, intensity, flicker, time);
                }
                if distance >= end_distance {
                    return [0.0, 0.0, 0.0];
                }

                let attenuation = if distance <= start_distance {
                    1.0
                } else {
                    // let attenuation =
                    //     1.0 - ((distance - start_distance) / (end_distance - start_distance));
                    //
                    smoothstep(end_distance, start_distance, distance)
                };
                let adjusted_intensity = intensity * attenuation;
                apply_flicker(color, adjusted_intensity, flicker, time)
            }
            Light::AmbientLight {
                color, intensity, ..
            } => {
                // Ambient light doesn't attenuate
                apply_flicker(color, intensity, None, time)
            }
            Light::Spotlight {
                position,
                direction,
                color,
                intensity,
                start_distance,
                end_distance,
                cone_angle,
                flicker,
            } => {
                let distance = (point - position).magnitude();
                if distance >= end_distance {
                    return [0.0, 0.0, 0.0];
                }

                let attenuation = if distance <= start_distance {
                    1.0
                } else {
                    1.0 - ((distance - start_distance) / (end_distance - start_distance))
                };

                // Spotlight cone angle falloff
                let direction_to_point = (point - position).normalized();
                let angle = direction.normalized().dot(direction_to_point).acos();
                if angle > cone_angle {
                    return [0.0, 0.0, 0.0];
                }

                let adjusted_intensity = intensity * attenuation;
                apply_flicker(color, adjusted_intensity, flicker, time)
            }
            Light::AreaLight {
                position,
                normal,
                width,
                height,
                color,
                intensity,
            } => {
                let area = width * height;
                let to_point = point - position;
                let distance = to_point.magnitude();

                if distance == 0.0 {
                    return [0.0, 0.0, 0.0];
                }

                let direction = to_point.normalized();
                let angle_attenuation = normal.normalized().dot(direction).max(0.0);
                let distance_attenuation = 1.0 / (distance * distance);
                let attenuation = angle_attenuation * distance_attenuation * area * intensity;

                [
                    color[0] * attenuation,
                    color[1] * attenuation,
                    color[2] * attenuation,
                ]
            }
        }
    }

    /// Sets the color of the light
    pub fn set_color(&mut self, new_color: [f32; 3]) {
        match self {
            Light::PointLight { color, .. } => *color = new_color,
            Light::AmbientLight { color, .. } => *color = new_color,
            Light::Spotlight { color, .. } => *color = new_color,
            Light::AreaLight { color, .. } => *color = new_color,
        }
    }

    /// Sets the intensity of the light
    pub fn set_intensity(&mut self, new_intensity: f32) {
        match self {
            Light::PointLight { intensity, .. } => *intensity = new_intensity,
            Light::AmbientLight { intensity, .. } => *intensity = new_intensity,
            Light::Spotlight { intensity, .. } => *intensity = new_intensity,
            Light::AreaLight { intensity, .. } => *intensity = new_intensity,
        }
    }

    /// Sets the start distance of the light for applicable light types
    pub fn set_start_distance(&mut self, new_start_distance: f32) {
        match self {
            Light::PointLight { start_distance, .. } => *start_distance = new_start_distance,
            Light::Spotlight { start_distance, .. } => *start_distance = new_start_distance,
            _ => {}
        }
    }

    /// Sets the end distance of the light for applicable light types
    pub fn set_end_distance(&mut self, new_end_distance: f32) {
        match self {
            Light::PointLight { end_distance, .. } => *end_distance = new_end_distance,
            Light::Spotlight { end_distance, .. } => *end_distance = new_end_distance,
            _ => {}
        }
    }
}

/// Applies flickering to the light color
fn apply_flicker(color: [f32; 3], intensity: f32, flicker: Option<Flicker>, time: f32) -> [f32; 3] {
    let flicker_factor = if let Some(flicker) = flicker {
        let noise = ((time * flicker.frequency).sin() * 0.5 + 0.5) * flicker.amplitude; // Sine-based flicker
        1.0 - noise // Reduce intensity by the flicker factor
    } else {
        1.0 // No flicker
    };

    [
        color[0] * intensity * flicker_factor,
        color[1] * intensity * flicker_factor,
        color[2] * intensity * flicker_factor,
    ]
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t) // Smooth cubic interpolation
}
