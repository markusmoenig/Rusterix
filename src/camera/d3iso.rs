use crate::{D3Camera, Ray};
use vek::{FrustumPlanes, Mat4, Vec2, Vec3};

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

    fn view_matrix(&self) -> Mat4<f32> {
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
        self.center + self.iso_dir * self.distance
    }

    fn create_ray(&self, uv: Vec2<f32>, screen: Vec2<f32>, jitter: Vec2<f32>) -> Ray {
        // constant orthographic direction
        let d = if self.tilted {
            self.tilt_dir // (-0.5,-1,-0.5).norm()
        } else {
            -self.iso_dir // ( 1,-1,-1).norm()
        };

        // film-plane basis
        let mut r = d.cross(Vec3::unit_y()); // right  (screen +X)
        if r.magnitude_squared() < 1e-6 {
            // extreme edge case guard
            r = Vec3::unit_x();
        }
        r = r.normalized();
        let s = r.cross(d).normalized(); // screen-up (+Y)

        // extents
        let half_h = self.scale;
        let half_w = half_h * (screen.x / screen.y);

        // slide origin across plane
        let o = Vec2::new(uv.x + jitter.x - 0.5, uv.y + jitter.y - 0.5);
        let origin = self.center + r * o.x * half_w * 2.0 + s * o.y * half_h * 2.0;

        Ray::new(origin, d) // d already normalised
    }
}
