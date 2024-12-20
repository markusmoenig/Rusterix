use vek::{Mat4, Vec2, Vec3};

use super::D3Camera;

pub struct D3OrbitCamera {
    pub target: Vec3<f32>, // The point the camera orbits around
    pub distance: f32,     // Distance from the target
    pub azimuth: f32,      // Horizontal rotation in radians
    pub elevation: f32,    // Vertical rotation in radians
    pub up: Vec3<f32>,     // Camera's "up" direction (usually Vec3::unit_y())
}

impl D3Camera for D3OrbitCamera {
    fn new() -> Self {
        Self {
            target: Vec3::zero(),
            distance: -1.5,
            azimuth: std::f32::consts::PI / 2.0,
            elevation: 0.0,
            up: Vec3::unit_y(),
        }
    }

    fn view_matrix(&self) -> Mat4<f32> {
        // Convert spherical coordinates to cartesian coordinates
        let x = self.distance * self.azimuth.cos() * self.elevation.cos();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.azimuth.sin() * self.elevation.cos();

        let position = Vec3::new(x, y, z) + self.target;

        Mat4::look_at_lh(position, self.target, self.up)
    }

    fn set_parameter_vec2(&mut self, key: &str, value: Vec2<f32>) {
        #[allow(clippy::single_match)]
        match key {
            "from_normalized" => {
                self.azimuth = std::f32::consts::PI * value.x;
                self.elevation = std::f32::consts::PI * (value.y - 0.5);
            }
            _ => {}
        }
    }
}
