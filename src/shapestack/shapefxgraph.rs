use crate::{ShapeContext, ShapeFX, ShapeFXRole};
use theframework::prelude::*;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeFXGraph {
    pub id: Uuid,
    pub effects: Vec<ShapeFX>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    pub selected_node: Option<usize>,

    pub scroll_offset: Vec2<i32>,
    pub zoom: f32,

    pub output: TheRGBABuffer,
}

impl Default for ShapeFXGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ShapeFXGraph {
    pub fn new() -> Self {
        let mut output = TheRGBABuffer::new(TheDim::sized(100, 100));
        output.fill([255, 255, 255, 255]);
        Self {
            id: Uuid::new_v4(),
            effects: vec![],
            connections: vec![],
            selected_node: None,
            scroll_offset: Vec2::zero(),
            zoom: 1.0,
            output,
        }
    }

    /// Evaluate the graph
    pub fn evaluate(&self, ctx: &ShapeContext) -> Option<Vec4<f32>> {
        for effect in self.effects.iter() {
            if effect.role == ShapeFXRole::Geometry {
                continue;
            }
            if let Some(col) = effect.evaluate(ctx) {
                return Some(col);
            }
        }
        None
    }
}
