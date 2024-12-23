use vek::{Mat4, Vec3};

use super::D3Camera;

pub struct D3FirstPCamera {
    pub position: Vec3<f32>,
    pub look_at: Vec3<f32>,
}

impl D3Camera for D3FirstPCamera {
    fn new() -> Self {
        Self {
            position: Vec3::zero(),
            look_at: Vec3::zero(),
        }
    }

    fn id(&self) -> String {
        "firstp".to_string()
    }

    fn view_matrix(&self) -> Mat4<f32> {
        let up = vek::Vec3::new(0.0, 1.0, 0.0);
        vek::Mat4::look_at_lh(self.position, self.look_at, up)
    }

    /*
    fn projection_matrix(
        &self,
        fov: f32,
        width: f32,
        height: f32,
        near: f32,
        far: f32,
    ) -> Mat4<f32> {
        vek::Mat4::perspective_fov_lh_no(fov, width, height, near, far)
    }*/

    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {
        #[allow(clippy::single_match)]
        match key {
            "position" => {
                self.position = value;
            }
            "look_at" => {
                self.look_at = value;
            }
            _ => {}
        }
    }
}
