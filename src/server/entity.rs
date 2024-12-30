use theframework::prelude::*;
use vek::{Vec2, Vec3};

use crate::prelude::*;

/// A game character.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Entity {
    /// The XZ orientation
    pub orientation: Vec2<f32>,
    /// The position in the map
    pub position: Vec3<f32>,
    /// The vertical camera tilt, 0.0 means flat, no tilt.
    pub tilt: f32,
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity {
    pub fn new() -> Self {
        Self {
            orientation: Vec2::new(1.0, 0.0),
            position: Vec3::new(0.0, 0.5, 0.0),
            tilt: 0.0,
        }
    }

    /// Computes the look-at target based on position, orientation, and vertical tilt (tilt).
    pub fn camera_look_at(&self) -> Vec3<f32> {
        let vertical_offset = self.orientation.magnitude() * self.tilt.sin();
        Vec3::new(
            self.position.x + self.orientation.x,
            self.position.y + vertical_offset,
            self.position.z + self.orientation.y,
        )
    }

    /// Rotates the entity to the left by a certain degree.
    pub fn turn_left(&mut self, degrees: f32) {
        self.rotate_orientation(-degrees.to_radians());
    }

    /// Rotates the entity to the right by a certain degree.
    pub fn turn_right(&mut self, degrees: f32) {
        self.rotate_orientation(degrees.to_radians());
    }

    /// Moves the entity forward along its current orientation.
    pub fn move_forward(&mut self, distance: f32) {
        self.position.x += self.orientation.x * distance;
        self.position.z += self.orientation.y * distance;
    }

    /// Moves the entity backward along its current orientation.
    pub fn move_backward(&mut self, distance: f32) {
        self.position.x -= self.orientation.x * distance;
        self.position.z -= self.orientation.y * distance;
    }

    /// Helper method to rotate the orientation vector by a given angle in radians.
    fn rotate_orientation(&mut self, radians: f32) {
        let cos_angle = radians.cos();
        let sin_angle = radians.sin();
        let new_x = self.orientation.x * cos_angle - self.orientation.y * sin_angle;
        let new_y = self.orientation.x * sin_angle + self.orientation.y * cos_angle;
        self.orientation = Vec2::new(new_x, new_y).normalized();
    }

    /// Maps a normalized screen coordinate (0.0 to 1.0) to a `tilt` angle.
    /// `0.0` -> maximum downward tilt, `1.0` -> maximum upward tilt.
    pub fn set_tilt_from_screen_coordinate(&mut self, screen_y: f32) {
        // Map the normalized screen coordinate to a range of angles (e.g., -π/4 to π/4)
        let max_tilt = std::f32::consts::FRAC_PI_4; // 45 degrees
        self.tilt = (screen_y - 0.5) * 2.0 * max_tilt;
    }

    /// Applies the camera's position and look-at parameters based on the entity's state.
    pub fn apply_to_camera(&self, camera: &mut Box<dyn D3Camera>) {
        // println!("{} {}", self.position, self.orientation);
        camera.set_parameter_vec3("position", self.position);
        camera.set_parameter_vec3("center", self.camera_look_at());
    }
}
