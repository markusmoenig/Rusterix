use vek::{FrustumPlanes, Mat4, Vec3};

use super::D3Camera;

pub struct D3IsoCamera {
    pub position: Vec3<f32>,
    pub center: Vec3<f32>,
}

impl D3Camera for D3IsoCamera {
    fn new() -> Self {
        Self {
            position: Vec3::zero(),
            center: Vec3::zero(),
        }
    }

    fn id(&self) -> String {
        "iso".to_string()
    }

    fn view_matrix(&self) -> Mat4<f32> {
        let up = vek::Vec3::new(0.0, 1.0, 0.0);
        vek::Mat4::look_at_lh(self.position, self.center, up)
    }

    fn projection_matrix(&self, width: f32, height: f32) -> Mat4<f32> {
        let scale = 2.0;
        let aspect_ratio = width / height;
        let left = -scale * aspect_ratio;
        let right = scale * aspect_ratio;
        let bottom = -scale;
        let top = scale;
        let near = -100.0;
        let far = 100.0;
        let orthographic_planes = FrustumPlanes {
            left,
            right,
            bottom,
            top,
            near,
            far,
        };
        vek::Mat4::orthographic_lh_no(orthographic_planes)
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
