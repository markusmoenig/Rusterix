use vek::{Mat4, Vec3};

use super::D3Camera;

pub struct D3FirstPCamera {
    pub position: Vec3<f32>,
    pub center: Vec3<f32>,

    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl D3Camera for D3FirstPCamera {
    fn new() -> Self {
        Self {
            position: Vec3::zero(),
            center: Vec3::zero(),

            fov: 75.0,
            near: 0.01,
            far: 100.0,
        }
    }

    fn id(&self) -> String {
        "firstp".to_string()
    }

    fn view_matrix(&self) -> Mat4<f32> {
        let up = vek::Vec3::new(0.0, 1.0, 0.0);
        vek::Mat4::look_at_lh(self.position, self.center, up)
    }

    fn projection_matrix(&self, width: f32, height: f32) -> Mat4<f32> {
        vek::Mat4::perspective_fov_lh_zo(self.fov.to_radians(), width, height, self.near, self.far)
    }

    fn set_parameter_f32(&mut self, key: &str, value: f32) {
        match key {
            "fov" => {
                self.fov = value;
            }
            "near" => {
                self.near = value;
            }
            "far" => {
                self.far = value;
            }
            _ => {}
        }
    }

    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {
        #[allow(clippy::single_match)]
        match key {
            "position" => {
                self.position = value;
            }
            "center" => {
                self.center = value;
            }
            _ => {}
        }
    }
}
