use crate::{D3Camera, Ray};
use vek::{FrustumPlanes, Mat4, Vec2, Vec3};

impl D3IsoCamera {
    #[inline]
    fn compute_dir_and_pos(&self) -> (Vec3<f32>, Vec3<f32>) {
        let (base, x_dir) = if self.tilted {
            (self.tilt_dir, -self.x_dir)
        } else {
            (self.iso_dir, self.x_dir)
        };
        let dir = Vec3::new(base.x * x_dir, base.y, base.z).normalized();
        let pos = if self.tilted {
            self.center - dir * self.distance
        } else {
            self.center + dir * self.distance
        };
        (dir, pos)
    }
}

#[derive(Clone)]
pub struct D3IsoCamera {
    pub center: Vec3<f32>,

    iso_dir: Vec3<f32>,
    tilt_dir: Vec3<f32>,

    pub distance: f32,
    pub scale: f32,

    pub tilted: bool,
    x_dir: f32, // (−1 = left, +1 = right)
}

impl D3Camera for D3IsoCamera {
    fn new() -> Self {
        Self {
            center: Vec3::zero(),

            iso_dir: Vec3::new(-1.0, 1.0, 1.0).normalized(),
            tilt_dir: Vec3::new(-0.5, -1.0, -0.5).normalized(),

            distance: 20.0,
            scale: 4.0,

            tilted: true,
            x_dir: 1.0, // right view
        }
    }

    fn id(&self) -> String {
        "iso".to_string()
    }

    /// Zoom the camera in or out based on vertical mouse delta
    fn zoom(&mut self, delta: f32) {
        let zoom_sensitivity = 0.05;

        let zoom_factor = (1.0 - delta * zoom_sensitivity).clamp(0.5, 2.0);

        self.scale *= zoom_factor;
        self.scale = self.scale.clamp(2.0, 70.0);
    }

    fn view_matrix(&self) -> Mat4<f32> {
        let (_dir, pos) = self.compute_dir_and_pos();
        Mat4::look_at_rh(pos, self.center, Vec3::unit_y())
    }

    fn projection_matrix(&self, width: f32, height: f32) -> Mat4<f32> {
        let half_h = self.scale;
        let half_w = half_h * (width / height);

        Mat4::orthographic_rh_no(FrustumPlanes {
            left: -half_w,
            right: half_w,
            bottom: -half_h,
            top: half_h,
            near: 0.1,
            far: 100.0,
        })
    }

    fn set_parameter_f32(&mut self, key: &str, value: f32) {
        #[allow(clippy::single_match)]
        match key {
            "scale" => {
                self.scale = value;
            }
            "distance" => self.distance = value.max(0.1),
            "right" => self.x_dir = 1.0,
            "left" => self.x_dir = -1.0,
            _ => {}
        }
    }

    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {
        #[allow(clippy::single_match)]
        match key {
            "center" => {
                self.center = value;
            }
            _ => {}
        }
    }

    fn position(&self) -> Vec3<f32> {
        let (_dir, pos) = self.compute_dir_and_pos();
        pos
    }

    fn create_ray(&self, uv: Vec2<f32>, screen: Vec2<f32>, jitter: Vec2<f32>) -> Ray {
        // extents
        let half_h = self.scale;
        let half_w = half_h * (screen.x / screen.y);

        // Match view_matrix()/position() exactly
        let cam_origin = self.position();

        let cam_look_at = self.center;

        let w = (cam_origin - cam_look_at).normalized();
        // Right-handed basis matching look_at_rh:
        // forward (into scene) = -w; right = forward × up; up' = right × forward
        let forward = (-w).normalized();
        let mut right = forward.cross(Vec3::unit_y());
        if right.magnitude_squared() < 1e-6 {
            right = Vec3::unit_x();
        }
        right = right.normalized();
        let up2 = right.cross(forward).normalized();

        let horizontal = right * half_w * 2.0;
        let vertical = up2 * half_h * 2.0;

        let pixel_size = Vec2::new(1.0 / screen.x, 1.0 / screen.y);

        let origin = cam_origin
            + horizontal * (pixel_size.x * jitter.x + uv.x - 0.5)
            + vertical * (pixel_size.y * jitter.y + uv.y - 0.5);

        Ray::new(origin, forward)
    }
}
