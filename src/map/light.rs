use vek::Vec3;

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
    DirectionalLight {
        direction: Vec3<f32>,
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
    /// Calculates the light's intensity and color at a given point
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

                let attenuation =
                    1.0 - ((distance - start_distance) / (end_distance - start_distance));
                let adjusted_intensity = intensity * attenuation;
                apply_flicker(color, adjusted_intensity, flicker, time)
            }
            Light::DirectionalLight {
                color, intensity, ..
            } => {
                // Directional light doesn't attenuate
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

                // Distance attenuation
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

                // Direction vector from the light to the point
                let direction = to_point.normalized();

                // Angle attenuation (dot product of normal and direction)
                let angle_attenuation = normal.normalized().dot(direction).max(0.0);

                // Distance attenuation (inverse square law)
                let distance_attenuation = 1.0 / (distance * distance);

                // Final intensity scaling
                let attenuation = angle_attenuation * distance_attenuation * area * intensity;

                [
                    color[0] * attenuation,
                    color[1] * attenuation,
                    color[2] * attenuation,
                ]
            }
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
