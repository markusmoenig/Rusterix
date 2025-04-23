use crate::{Pixel, ShapeContext, ShapeFX, ShapeFXRole, Texture};
use rayon::prelude::*;
use theframework::prelude::*;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeFXGraph {
    pub id: Uuid,
    pub nodes: Vec<ShapeFX>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    pub selected_node: Option<usize>,

    pub scroll_offset: Vec2<i32>,
    pub zoom: f32,
}

impl Default for ShapeFXGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ShapeFXGraph {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            nodes: vec![],
            connections: vec![],
            selected_node: None,
            scroll_offset: Vec2::zero(),
            zoom: 1.0,
        }
    }

    /// Evaluate the graph
    pub fn evaluate(
        &self,
        ctx: &ShapeContext,
        mut incoming: Vec4<f32>,
        palette: &ThePalette,
    ) -> Option<Vec4<f32>> {
        // for effect in self.effects.iter() {
        //     if effect.role == ShapeFXRole::Geometry {
        //         continue;
        //     }
        //     if let Some(col) = self.effects[1].evaluate(ctx, palette) {
        //         return Some(col);
        //     }
        // }
        if self.nodes.is_empty() {
            return None;
        }
        if self.nodes[0].role != ShapeFXRole::Geometry {
            return None;
        }

        let mut curr_index = 0_usize;
        let mut curr_terminal = if ctx.distance > 0.0 { 1_usize } else { 0_usize };

        let mut color = None;

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                if let Some(col) =
                    self.nodes[next_node as usize].evaluate(ctx, Some(incoming), palette)
                {
                    color = Some(col);
                    incoming = col;
                }
                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }
        color
    }

    /// Returns the connected input node and terminal for the given output node and terminal.
    pub fn find_connected_input_node(
        &self,
        node: usize,
        terminal_index: usize,
    ) -> Option<(u16, u8)> {
        for (o, ot, i, it) in &self.connections {
            if *o == node as u16 && *ot == terminal_index as u8 {
                return Some((*i, *it));
            }
        }
        None
    }

    /// Returns the connected output node for the given input node and terminal.
    pub fn find_connected_output_node(&self, node: usize, terminal_index: usize) -> Option<usize> {
        for (o, _, i, it) in &self.connections {
            if *i == node as u16 && *it == terminal_index as u8 {
                return Some(*o as usize);
            }
        }
        None
    }

    /// Create a preview of the graph
    pub fn preview(&self, buffer: &mut Texture, palette: &ThePalette) {
        let width = buffer.width;
        let height = buffer.height;

        let px = 1.0;

        buffer
            .data
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let x = i as f32;
                    let y = j as f32;

                    // Normalized UVs
                    let uv = Vec2::new(x / width as f32, 1.0 - y / height as f32);

                    // Centered pixel coordinate in "world space"
                    let world = uv * Vec2::new(width as f32, height as f32);

                    // Simulated distance to nearest edge of the preview "shape"
                    let dist_left = uv.x;
                    let dist_right = 1.0 - uv.x;
                    let dist_top = 1.0 - uv.y;
                    let dist_bottom = uv.y;
                    let edge_distance = dist_left.min(dist_right).min(dist_top).min(dist_bottom);

                    // Optional: scale to world/pixel units if needed
                    let distance = -edge_distance * width.min(height) as f32;

                    // Build ShapeContext with no sector
                    let ctx = ShapeContext {
                        point_world: world,
                        point: world / px,
                        uv,
                        distance_world: distance,
                        distance,
                        shape_id: 0,
                        px,
                        anti_aliasing: 1.0,
                        t: None,
                        line_dir: None,
                    };

                    let color = if let Some(col) =
                        self.evaluate(&ctx, Vec4::new(0.0, 0.0, 0.0, 1.0), palette)
                    {
                        col
                    } else {
                        Vec4::new(0.0, 0.0, 0.0, 1.0)
                    };

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });
    }

    /// Get the dominant color of the graph for sector previews
    pub fn get_dominant_color(&self, palette: &ThePalette) -> Pixel {
        let mut pixel = [128, 128, 128, 255];
        if self.nodes.len() > 1 {
            pixel = self.nodes[1].get_dominant_color(palette)
        }
        pixel
    }
}
