use vek::{Mat4, Vec2, Vec3};

use super::D3Camera;

pub struct D3OrbitCamera {
    pub center: Vec3<f32>,
    pub distance: f32,
    pub azimuth: f32,
    pub elevation: f32,
    pub up: Vec3<f32>,

    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl D3Camera for D3OrbitCamera {
    fn new() -> Self {
        Self {
            center: Vec3::zero(),
            distance: 1.5,
            azimuth: std::f32::consts::PI / 2.0,
            elevation: 0.0,
            up: Vec3::unit_y(),

            fov: 75.0,
            near: 0.01,
            far: 100.0,
        }
    }

    fn id(&self) -> String {
        "orbit".to_string()
    }

    fn view_matrix(&self) -> Mat4<f32> {
        // Convert spherical coordinates to cartesian coordinates
        let x = self.distance * self.azimuth.cos() * self.elevation.cos();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.azimuth.sin() * self.elevation.cos();

        let position = Vec3::new(x, y, z) + self.center;

        Mat4::look_at_rh(position, self.center, self.up)
    }

    fn projection_matrix(&self, width: f32, height: f32) -> Mat4<f32> {
        vek::Mat4::perspective_fov_rh_zo(self.fov.to_radians(), width, height, self.near, self.far)
    }

    fn set_parameter_f32(&mut self, key: &str, value: f32) {
        #[allow(clippy::single_match)]
        match key {
            "distance" => {
                self.distance = value;
            }
            _ => {}
        }
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
