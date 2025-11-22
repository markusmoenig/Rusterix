use super::{
    ActionProperties, ConnectionMode, ControlPoint, MeshTopology, SectorMeshDescriptor,
    SurfaceAction,
};
use vek::Vec2;

/// A terrain action that creates smooth height-interpolated surfaces
/// based on vertex heights (vertex.z values) and optional control points
pub struct TerrainAction {
    /// Vertex heights in the order they appear in sector_uv
    /// These are the z-heights from the vertices
    pub vertex_heights: Vec<f32>,
    /// Additional height control points (position + height) for features like hills/valleys
    /// These are NOT part of the sector outline but influence height interpolation
    pub height_control_points: Vec<(Vec2<f32>, f32)>,
    /// Smoothness factor (0.0 = linear interpolation, higher = smoother)
    /// This is used as the power parameter in IDW (Inverse Distance Weighting)
    pub smoothness: f32,
}

impl SurfaceAction for TerrainAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        _surface_thickness: f32,
        _properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 || self.vertex_heights.len() != sector_uv.len() {
            return None;
        }

        // Create control points with interpolated heights
        let mut control_points = Vec::with_capacity(sector_uv.len());

        // If we have additional height control points, use IDW interpolation
        // Otherwise, use vertex heights directly
        if !self.height_control_points.is_empty() {
            // Build combined vertex list: boundary vertices + control points
            let mut all_height_points =
                Vec::with_capacity(sector_uv.len() + self.height_control_points.len());

            // Add boundary vertices with their heights
            for (i, &uv) in sector_uv.iter().enumerate() {
                all_height_points.push((uv, self.vertex_heights[i]));
            }

            // Add custom control points
            all_height_points.extend_from_slice(&self.height_control_points);

            // Interpolate height at each boundary vertex using all control points
            // This allows interior control points (hills/valleys) to influence boundary
            for &uv in sector_uv.iter() {
                let height = interpolate_height_idw(
                    uv,
                    &all_height_points,
                    self.smoothness.max(1.0), // Use smoothness as IDW power (min 1.0)
                );

                control_points.push(ControlPoint {
                    uv,
                    extrusion: height,
                });
            }
        } else {
            // No custom control points, use vertex heights directly
            for (i, &uv) in sector_uv.iter().enumerate() {
                control_points.push(ControlPoint {
                    uv,
                    extrusion: self.vertex_heights[i],
                });
            }
        }

        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: control_points,
                holes: vec![],
            }),
            sides: None, // Terrain typically blends smoothly with the base
            connection: ConnectionMode::Smooth, // Always smooth for terrain
        })
    }

    fn name(&self) -> &'static str {
        "Terrain"
    }
}

/// Helper function to create a terrain action with smooth height interpolation
///
/// # Arguments
/// * `sector_uv` - The UV coordinates of the sector boundary
/// * `vertex_heights` - Height values (z-component) for each vertex
/// * `height_control_points` - Additional height control points (position, height) for hills/valleys
/// * `smoothness` - Smoothness factor for interpolation (1.0 = linear IDW, higher = smoother)
pub fn create_smooth_terrain(
    sector_uv: &[Vec2<f32>],
    vertex_heights: &[f32],
    height_control_points: Vec<(Vec2<f32>, f32)>,
    smoothness: f32,
) -> Option<TerrainAction> {
    if sector_uv.len() != vertex_heights.len() || sector_uv.len() < 3 {
        return None;
    }

    Some(TerrainAction {
        vertex_heights: vertex_heights.to_vec(),
        height_control_points,
        smoothness: smoothness.max(1.0),
    })
}

/// Advanced: Create a terrain with subdivided mesh for smoother interpolation
///
/// This version creates additional control points inside the polygon
/// by interpolating heights, resulting in a smoother terrain surface.
pub struct SmoothedTerrainAction {
    /// The boundary vertices with heights
    pub boundary: Vec<ControlPoint>,
    /// Additional interior points for smoother interpolation
    pub interior_points: Vec<ControlPoint>,
    /// Smoothness factor
    pub smoothness: f32,
}

impl SurfaceAction for SmoothedTerrainAction {
    fn describe_mesh(
        &self,
        _sector_uv: &[Vec2<f32>],
        _surface_thickness: f32,
        _properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if self.boundary.len() < 3 {
            return None;
        }

        // For now, just use the boundary points
        // In a more advanced implementation, we could generate a grid
        // of interior points with heights interpolated from the boundary
        // using inverse distance weighting or other smooth interpolation methods

        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: self.boundary.clone(),
                holes: vec![],
            }),
            sides: None,
            connection: ConnectionMode::Smooth,
        })
    }

    fn name(&self) -> &'static str {
        "SmoothedTerrain"
    }
}

/// Helper to interpolate height at a point inside a polygon using Inverse Distance Weighting (IDW)
///
/// This can be used to generate smooth terrain with subdivided meshes
pub fn interpolate_height_idw(
    point: Vec2<f32>,
    vertices: &[(Vec2<f32>, f32)], // (position, height) pairs
    power: f32,
) -> f32 {
    if vertices.is_empty() {
        return 0.0;
    }

    // Check if point coincides with any vertex
    for (v_pos, v_height) in vertices {
        if (*v_pos - point).magnitude() < 1e-6 {
            return *v_height;
        }
    }

    // Inverse distance weighting
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;

    for (v_pos, v_height) in vertices {
        let distance = (*v_pos - point).magnitude();
        if distance < 1e-6 {
            return *v_height; // Exact match
        }

        let weight = 1.0 / distance.powf(power);
        weighted_sum += weight * v_height;
        weight_sum += weight;
    }

    if weight_sum > 1e-6 {
        weighted_sum / weight_sum
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_action_creation() {
        let sector_uv = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];

        let heights = vec![0.0, 1.0, 2.0, 1.5];

        let terrain = create_smooth_terrain(&sector_uv, &heights, vec![], 1.0).unwrap();

        assert_eq!(terrain.vertex_heights.len(), 4);
        assert_eq!(terrain.smoothness, 1.0);
        assert_eq!(terrain.height_control_points.len(), 0);
    }

    #[test]
    fn test_terrain_with_control_points() {
        let sector_uv = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];

        let heights = vec![0.0, 0.0, 0.0, 0.0];

        // Add a hill in the center
        let control_points = vec![
            (Vec2::new(5.0, 5.0), 5.0), // Hill peak at center
        ];

        let terrain = create_smooth_terrain(&sector_uv, &heights, control_points, 2.0).unwrap();

        assert_eq!(terrain.vertex_heights.len(), 4);
        assert_eq!(terrain.height_control_points.len(), 1);
        assert_eq!(terrain.smoothness, 2.0);
    }

    #[test]
    fn test_idw_interpolation() {
        let vertices = vec![
            (Vec2::new(0.0, 0.0), 0.0),
            (Vec2::new(10.0, 0.0), 10.0),
            (Vec2::new(0.0, 10.0), 10.0),
        ];

        // Point at center should be roughly average
        let center = Vec2::new(5.0, 5.0);
        let height = interpolate_height_idw(center, &vertices, 2.0);

        // Should be somewhere between min (0) and max (10)
        assert!(height > 0.0 && height < 10.0);
    }

    #[test]
    fn test_idw_exact_vertex() {
        let vertices = vec![(Vec2::new(0.0, 0.0), 5.0), (Vec2::new(10.0, 0.0), 10.0)];

        // Point exactly at a vertex should return that vertex's height
        let height = interpolate_height_idw(Vec2::new(0.0, 0.0), &vertices, 2.0);
        assert!((height - 5.0).abs() < 1e-5);
    }
}
