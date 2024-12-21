pub mod d2preview;

use crate::{Map, MapToolType, Scene, Texture, Tile};
use theframework::prelude::*;
use vek::Vec2;

#[allow(unused)]
pub trait SceneBuilder: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn build(
        &self,
        map: &Map,
        tiles: &FxHashMap<Uuid, Tile>,
        atlas: Texture,
        screen_size: Vec2<f32>,
    ) -> Scene;

    /// Convert a map grid position to screen coordinates
    fn map_grid_to_local(
        &self,
        screen_size: Vec2<f32>,
        grid_pos: Vec2<f32>,
        map: &Map,
    ) -> Vec2<f32> {
        let grid_space_pos = grid_pos * map.grid_size;
        grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
    }

    /// Set the current tool type, only needed for previews of visual editors. Used by D2PreviewBuilder.
    fn set_map_tool_type(&mut self, tool: MapToolType) {}

    /// Set the current hover info, only needed for previews of visual editors. Used by D2PreviewBuilder.
    fn set_map_hover_info(
        &mut self,
        hover: (Option<u32>, Option<u32>, Option<u32>),
        hover_cursor: Option<Vec2<f32>>,
    ) {
    }

    /// Set the camera info, only needed for previews of visual editors. Used by D2PreviewBuilder.
    fn set_camera_info(&mut self, pos: Option<vek::Vec3<f32>>, look_at: vek::Vec3<f32>) {}
}
