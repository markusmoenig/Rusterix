use vek::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct ShapeContext {
    /// World-space coordinate of this pixel
    pub point_world: Vec2<f32>,

    /// Pixel space coordinate of this pixel
    pub point: Vec2<f32>,

    /// UV relative to the shape's bounding box (0–1 range ideally)
    pub uv: Vec2<f32>,

    /// Signed distance to the shape edge (in world units)
    pub distance_world: f32,

    /// Signed distance to the shape edge (in pixel units)
    pub distance: f32,

    /// ID of the shape (usually sector.id)
    pub shape_id: u32,

    /// World-space size of a single pixel (for outline thickness, etc.)
    pub px: f32,
}
