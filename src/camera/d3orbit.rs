use vek::{Mat4, Vec2, Vec3};

use super::{D3Camera, Ray};

#[derive(Clone)]
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
            distance: 20.0,
            azimuth: std::f32::consts::PI / 2.0,
            elevation: 0.698,
            up: Vec3::unit_y(),

            fov: 75.0,
            near: 0.01,
            far: 100.0,
        }
    }

    fn id(&self) -> String {
        "orbit".to_string()
    }

    fn fov(&self) -> f32 {
        self.fov
    }

    fn distance(&self) -> f32 {
        self.distance
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

    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {
        #[allow(clippy::single_match)]
        match key {
            "center" => {
                self.center = value;
            }
            _ => {}
        }
    }

    /// Rotate the camera around its center point using mouse delta in screen space.
    /// delta: mouse delta in pixels (x, y)
    fn rotate(&mut self, delta: Vec2<f32>) {
        // Sensitivity values (tweak as needed)
        let sensitivity = 0.005;

        self.azimuth -= delta.x * sensitivity;
        self.elevation += delta.y * sensitivity;

        // Clamp elevation to avoid flipping (just below ±90°)
        let epsilon = 0.01;
        let max_elevation = std::f32::consts::FRAC_PI_2 - epsilon;
        self.elevation = self.elevation.clamp(-max_elevation, max_elevation);
    }

    /// Zoom the camera in or out based on vertical mouse delta
    fn zoom(&mut self, delta: f32) {
        let zoom_sensitivity = 0.05;

        // Compute zoom factor (make sure it's always > 0)
        let zoom_factor = (1.0 - delta * zoom_sensitivity).clamp(0.5, 2.0);

        self.distance *= zoom_factor;

        self.distance = self.distance.clamp(0.1, 100.0);
    }

    /// Create a ray from a screen-space UV coordinate and offset.
    fn create_ray(&self, uv: Vec2<f32>, screen: Vec2<f32>, offset: Vec2<f32>) -> Ray {
        let aspect = screen.x / screen.y;
        let pixel_size = Vec2::new(1.0 / screen.x, 1.0 / screen.y);

        let mut uv = uv;
        uv.y = 1.0 - uv.y;

        // Orbit camera position
        let x = self.distance * self.azimuth.cos() * self.elevation.cos();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.azimuth.sin() * self.elevation.cos();
        let position = Vec3::new(x, y, z) + self.center;

        // Compute correct basis
        let forward = (self.center - position).normalized(); // from eye to center
        let right = forward.cross(self.up).normalized();
        let up = right.cross(forward);

        // Screen plane height/width
        let half_height = (self.fov.to_radians() * 0.5).tan();
        let half_width = half_height * aspect;

        // Now build the ray
        let pixel_ndc = Vec2::new(
            (pixel_size.x * offset.x + uv.x) * 2.0 - 1.0, // [-1..1]
            (pixel_size.y * offset.y + uv.y) * 2.0 - 1.0,
        );

        let dir = (forward + right * pixel_ndc.x * half_width - up * pixel_ndc.y * half_height) // ← minus Y because screen Y usually goes down
            .normalized();

        Ray {
            origin: position,
            dir,
        }
    }

    fn basis_vectors(&self) -> (Vec3<f32>, Vec3<f32>, Vec3<f32>) {
        let x = self.distance * self.azimuth.cos() * self.elevation.cos();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.azimuth.sin() * self.elevation.cos();
        let position = Vec3::new(x, y, z) + self.center;

        let forward = (self.center - position).normalized();
        let right = forward.cross(self.up).normalized();
        let up = right.cross(forward).normalized();
        (forward, right, up)
    }
}
