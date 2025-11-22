use super::{
    ActionProperties, ConnectionMode, ControlPoint, MeshTopology, SectorMeshDescriptor,
    SurfaceAction,
};
use vek::Vec2;

/// A ridge action that creates an elevated flat platform with sloped sides
/// Perfect for iso-style terrain features like plateaus, raised walkways, etc.
pub struct RidgeAction;

impl SurfaceAction for RidgeAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        _surface_thickness: f32,
        properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 {
            return None;
        }

        let height = properties.height;
        let slope_width = properties.slope_width.max(0.01); // Minimum slope width

        // Create the flat top surface at the specified height
        let top_control_points: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: height,
            })
            .collect();

        // Create inset points for the slope transition
        // We'll inset the polygon by slope_width and create a sloped transition
        let center = calculate_polygon_center(sector_uv);

        let mut slope_base_points = Vec::with_capacity(sector_uv.len());
        for &uv in sector_uv {
            // Inset each vertex toward center by slope_width
            let dir = (uv - center).normalized();
            let inset_uv = uv - dir * slope_width;
            slope_base_points.push(ControlPoint {
                uv: inset_uv,
                extrusion: 0.0, // Base level
            });
        }

        // Create the mesh topology:
        // - Top cap: flat elevated surface
        // - Sides: sloped transition from base to top
        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: top_control_points.clone(),
                holes: vec![],
            }),
            sides: Some(MeshTopology::QuadStrip {
                loop_a: slope_base_points,
                loop_b: top_control_points,
            }),
            connection: ConnectionMode::Smooth, // Smooth blending at base
        })
    }

    fn name(&self) -> &'static str {
        "Ridge"
    }
}

/// Calculate the center point of a polygon
fn calculate_polygon_center(points: &[Vec2<f32>]) -> Vec2<f32> {
    if points.is_empty() {
        return Vec2::zero();
    }

    let sum = points.iter().fold(Vec2::zero(), |acc, &p| acc + p);
    sum / points.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ridge_action_creation() {
        let sector_uv = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];

        let properties = ActionProperties::default()
            .with_height(5.0)
            .with_slope_width(2.0);

        let ridge = RidgeAction;
        let descriptor = ridge.describe_mesh(&sector_uv, 1.0, &properties);

        assert!(descriptor.is_some());
        let desc = descriptor.unwrap();
        assert!(!desc.is_hole);
        assert!(desc.cap.is_some());
        assert!(desc.sides.is_some());
    }

    #[test]
    fn test_polygon_center() {
        let points = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];

        let center = calculate_polygon_center(&points);
        assert!((center.x - 5.0).abs() < 1e-5);
        assert!((center.y - 5.0).abs() < 1e-5);
    }
}
