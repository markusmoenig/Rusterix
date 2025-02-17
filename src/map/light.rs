use crate::{Value, ValueContainer};
use theframework::prelude::*;
use vek::{Vec2, Vec3};

/// Parameters for flickering
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LightType {
    Point,
    Ambient,
    Spot,
    Area,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub light_type: LightType,
    pub properties: ValueContainer,
}

impl Light {
    pub fn new(light_type: LightType) -> Self {
        Self {
            light_type,
            properties: ValueContainer::default(),
        }
    }

    /// Helper: get the position from the ValueContainer (defaults to [0,0,0] if not found)
    fn get_position(&self) -> Vec3<f32> {
        let p = self
            .properties
            .get_vec3("position")
            .unwrap_or([0.0, 0.0, 0.0]);
        Vec3::new(p[0], p[1], p[2])
    }

    /// Helper: get color (defaults to white if not found)
    pub fn get_color(&self) -> [f32; 3] {
        self.properties.get_vec3("color").unwrap_or([1.0, 1.0, 1.0])
    }

    /// Helper: get intensity (defaults to 1.0 if not found)
    pub fn get_intensity(&self) -> f32 {
        self.properties.get_float_default("intensity", 1.0)
    }

    /// Helper: get start distance (defaults to 3.0 if not found)
    pub fn get_start_distance(&self) -> f32 {
        self.properties.get_float_default("start_distance", 3.0)
    }

    /// Helper: get end distance (defaults to 5.0 if not found)
    pub fn get_end_distance(&self) -> f32 {
        self.properties.get_float_default("end_distance", 10.0)
    }

    /// Helper: get flicker
    pub fn get_flicker(&self) -> f32 {
        self.properties.get_float_default("flicker", 0.0)
    }

    /// Returns the position of the light (3D)
    pub fn position(&self) -> Vec3<f32> {
        self.get_position()
    }

    /// Returns the position of the light in 2D (x, z)
    pub fn position_2d(&self) -> Vec2<f32> {
        let p = self.position();
        Vec2::new(p.x, p.z)
    }

    /// Loads and caches all the parameters from the value container into a CompiledLight.
    pub fn compile(&self) -> CompiledLight {
        // Common parameters
        let position = {
            let p = self
                .properties
                .get_vec3("position")
                .unwrap_or([0.0, 0.0, 0.0]);
            Vec3::new(p[0], p[1], p[2])
        };
        let color = self.properties.get_vec3("color").unwrap_or([1.0, 1.0, 1.0]);
        let intensity = self.properties.get_float_default("intensity", 1.0);

        // For Point and Spot lights (if used)
        let start_distance = self.properties.get_float_default("start_distance", 3.0);
        let end_distance = self.properties.get_float_default("end_distance", 10.0);

        let flicker = self.properties.get_float_default("flicker", 0.0);

        // For spot lights:
        let direction = {
            let d = self
                .properties
                .get_vec3("direction")
                .unwrap_or([0.0, 0.0, -1.0]);
            Vec3::new(d[0], d[1], d[2]).normalized()
        };
        let cone_angle = self
            .properties
            .get_float_default("cone_angle", std::f32::consts::FRAC_PI_4);

        // For area lights:
        let normal = {
            let n = self
                .properties
                .get_vec3("normal")
                .unwrap_or([0.0, 1.0, 0.0]);
            Vec3::new(n[0], n[1], n[2]).normalized()
        };
        let width = self.properties.get_float_default("width", 1.0);
        let height = self.properties.get_float_default("height", 1.0);
        let emitting = self.properties.get_bool_default("emitting", true);

        CompiledLight {
            light_type: self.light_type,
            // common
            position,
            color,
            intensity,
            emitting,
            // point/spot
            start_distance,
            end_distance,
            flicker,
            // spot
            direction,
            cone_angle,
            // area
            normal,
            width,
            height,
        }
    }

    /// Set the position of the light
    pub fn set_position(&mut self, position: Vec3<f32>) {
        self.properties.set(
            "position",
            Value::Vec3([position.x, position.y, position.z]),
        );
    }

    /// Sets the color of the light
    pub fn set_color(&mut self, new_color: [f32; 3]) {
        self.properties.set("color", Value::Vec3(new_color));
    }

    /// Sets the intensity of the light
    pub fn set_intensity(&mut self, new_intensity: f32) {
        self.properties
            .set("intensity", Value::Float(new_intensity));
    }

    /// Sets the start distance (for Point or Spot)
    pub fn set_start_distance(&mut self, new_start_distance: f32) {
        self.properties
            .set("start_distance", Value::Float(new_start_distance));
    }

    /// Sets the end distance (for Point or Spot)
    pub fn set_end_distance(&mut self, new_end_distance: f32) {
        self.properties
            .set("end_distance", Value::Float(new_end_distance));
    }

    /// Set flicker frequency and amplitude
    pub fn set_flicker(&mut self, flicker: f32) {
        self.properties.set("flicker", Value::Float(flicker));
    }
}

/// A “compiled” version of Light that caches all values needed for rendering.
#[derive(Debug, Clone)]
pub struct CompiledLight {
    pub light_type: LightType,
    // common parameters
    pub position: Vec3<f32>,
    pub color: [f32; 3],
    pub intensity: f32,
    pub emitting: bool,
    // for point and spot lights
    pub start_distance: f32,
    pub end_distance: f32,
    pub flicker: f32,
    // for spot lights
    pub direction: Vec3<f32>,
    pub cone_angle: f32,
    // for area lights
    pub normal: Vec3<f32>,
    pub width: f32,
    pub height: f32,
}

impl CompiledLight {
    /// Returns the 3D position of the light.
    pub fn position(&self) -> Vec3<f32> {
        self.position
    }

    /// Returns the 2D position of the light (x, z).
    pub fn position_2d(&self) -> Vec2<f32> {
        Vec2::new(self.position.x, self.position.z)
    }

    /// Calculate the light's intensity and color at a given point.
    pub fn color_at(&self, point: Vec3<f32>, hash: &u32) -> Option<[f32; 3]> {
        if !self.emitting {
            return None;
        };
        match self.light_type {
            LightType::Point => self.calculate_point_light(point, hash),
            LightType::Ambient => self.calculate_ambient_light(hash),
            LightType::Spot => self.calculate_spot_light(point, hash),
            LightType::Area => self.calculate_area_light(point, hash),
        }
    }

    fn calculate_point_light(&self, point: Vec3<f32>, hash: &u32) -> Option<[f32; 3]> {
        let distance = (point - self.position).magnitude();

        // Beyond end_distance => no intensity
        if distance >= self.end_distance {
            return None;
        }

        // Within start_distance => full intensity
        if distance <= self.start_distance {
            return Some(self.apply_flicker(self.color, self.intensity, self.flicker, hash));
        }

        // Smooth attenuation between start and end
        let attenuation = self.smoothstep(self.end_distance, self.start_distance, distance);
        let adjusted_intensity = self.intensity * attenuation;
        Some(self.apply_flicker(self.color, adjusted_intensity, self.flicker, hash))
    }

    fn calculate_ambient_light(&self, hash: &u32) -> Option<[f32; 3]> {
        // Ambient light does not attenuate by distance.
        Some(self.apply_flicker(self.color, self.intensity, self.flicker, hash))
    }

    fn calculate_spot_light(&self, point: Vec3<f32>, hash: &u32) -> Option<[f32; 3]> {
        let distance = (point - self.position).magnitude();
        if distance >= self.end_distance {
            return None;
        }

        let attenuation = if distance <= self.start_distance {
            1.0
        } else {
            1.0 - ((distance - self.start_distance) / (self.end_distance - self.start_distance))
        };

        // Check if the point is within the spot cone
        let direction_to_point = (point - self.position).normalized();
        let angle = self.direction.dot(direction_to_point).acos();
        if angle > self.cone_angle {
            return None;
        }

        let adjusted_intensity = self.intensity * attenuation;
        Some(self.apply_flicker(self.color, adjusted_intensity, self.flicker, hash))
    }

    fn calculate_area_light(&self, point: Vec3<f32>, _hash: &u32) -> Option<[f32; 3]> {
        let to_point = point - self.position;
        let distance = to_point.magnitude();
        if distance == 0.0 {
            return None;
        }

        let direction = to_point.normalized();
        let angle_attenuation = self.normal.dot(direction).max(0.0);
        let distance_attenuation = 1.0 / (distance * distance);
        let area = self.width * self.height;

        let attenuation = angle_attenuation * distance_attenuation * area * self.intensity;
        Some([
            self.color[0] * attenuation,
            self.color[1] * attenuation,
            self.color[2] * attenuation,
        ])
    }

    /// Applies flicker effect to the light color.
    fn apply_flicker(&self, color: [f32; 3], intensity: f32, flicker: f32, hash: &u32) -> [f32; 3] {
        let flicker_factor = if flicker > 0.0 {
            let combined_hash = hash.wrapping_add(
                (self.position.x as u32 + self.position.y as u32 + self.position.z as u32) * 100,
            );
            let flicker_value = (combined_hash as f32 / u32::MAX as f32).clamp(0.0, 1.0);
            1.0 - flicker_value * flicker
        } else {
            1.0
        };

        [
            color[0] * intensity * flicker_factor,
            color[1] * intensity * flicker_factor,
            color[2] * intensity * flicker_factor,
        ]
    }

    fn smoothstep(&self, edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}
