pub mod d3firstp;
pub mod d3iso;
pub mod d3orbit;

use vek::{Mat4, Vec2, Vec3, Vec4};

#[allow(unused)]
pub trait D3Camera: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> String {
        "".to_string()
    }

    fn view_matrix(&self) -> Mat4<f32> {
        Mat4::identity()
    }

    fn projection_matrix(
        &self,
        fov: f32,
        width: f32,
        height: f32,
        near: f32,
        far: f32,
    ) -> Mat4<f32> {
        Mat4::perspective_fov_lh_zo(fov.to_radians(), width, height, near, far)
    }

    /// Set an f32 parameter.
    fn set_parameter_f32(&mut self, key: &str, value: f32) {}

    /// Set a Vec2 parameter.
    fn set_parameter_vec2(&mut self, key: &str, value: Vec2<f32>) {}

    /// Set a Vec3 parameter.
    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {}

    /// Set a Vec4 parameter.
    fn set_parameter_vec4(&mut self, key: &str, value: Vec4<f32>) {}
}
