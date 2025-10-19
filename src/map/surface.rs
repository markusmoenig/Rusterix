use crate::{Map, Sector};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::{Vec2, Vec3};

use earcutr::earcut;

/// Operation applied to a profile loop on this surface (non-destructive).
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum LoopOp {
    None,
    Relief { height: f32 }, // positive outward along surface normal
    Recess { depth: f32 },  // positive inward along surface normal
}

/// One closed loop in the surface's UV/profile space.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileLoop {
    pub path: Vec<Vec2<f32>>, // points in UV space, assumed to be simple polygon
    pub op: LoopOp,           // optional loop-specific op
    /// The profile-map sector this loop came from. `None` for the outer host loop.
    pub origin_profile_sector: Option<u32>,
}

/// Represents a geometric plane defined by an origin and a normal vector.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Plane {
    pub origin: Vec3<f32>,
    pub normal: Vec3<f32>,
}

/// Represents a 3D basis with right, up, and normal vectors.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Basis3 {
    pub right: Vec3<f32>,
    pub up: Vec3<f32>,
    pub normal: Vec3<f32>,
}

/// Defines an editable plane with origin, axes for 2D editing, and a scale factor.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct EditPlane {
    pub origin: Vec3<f32>,
    pub right: Vec3<f32>,
    pub up: Vec3<f32>,
    pub scale: f32,
}

/// Represents an attachment with a transform relative to a surface and optional mesh or procedural references.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Attachment {
    pub id: Uuid,
    pub surface_id: Uuid,
    pub transform: [[f32; 4]; 4],
    pub mesh_ref: Option<Uuid>,
    pub proc_ref: Option<Uuid>,
}

/// Represents a surface with the sector owner, geometry, and profile.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Surface {
    pub id: Uuid,
    pub sector_id: u32,

    /// Geometric frame of the editable plane for this surface
    pub plane: Plane,
    pub frame: Basis3,
    pub edit_uv: EditPlane,

    /// Uuid of the Profile
    pub profile: Option<Uuid>,

    /// Optional, the vertices of the surface in world coordinates, used in cases where we need to pass standalone surfaces.
    #[serde(skip)]
    pub world_vertices: Vec<Vec3<f32>>,
}

impl Surface {
    pub fn new(sector_id: u32) -> Surface {
        Surface {
            id: Uuid::new_v4(),
            sector_id,
            plane: Plane::default(),
            frame: Basis3::default(),
            edit_uv: EditPlane::default(),
            profile: None,
            world_vertices: vec![],
        }
    }

    /// Calculate the geometry
    pub fn calculate_geometry(&mut self, map: &Map) {
        if let Some(sector) = map.find_sector(self.sector_id) {
            if let Some(points) = sector.vertices_world(map) {
                // existing logic using `points`
                let (centroid, mut normal) = newell_plane(&points);
                if normal.magnitude() < 1e-6 {
                    normal = Vec3::new(0.0, 1.0, 0.0);
                }
                let mut right = stable_right(&points, normal);
                let mut up = normalize_or_zero(normal.cross(right));

                if up.magnitude() < 1e-6 {
                    // fallback: try swapping axes
                    right = normalize_or_zero(normal.cross(Vec3::new(0.0, 1.0, 0.0)));
                    up = normalize_or_zero(normal.cross(right));
                }

                if up.magnitude() < 1e-6 {
                    // final fallback
                    right = Vec3::new(1.0, 0.0, 0.0);
                    up = normalize_or_zero(normal.cross(right));
                }

                // ensure orthonormal basis (flip right if needed)
                let test_up = normalize_or_zero(normal.cross(right));
                if test_up.magnitude() > 1e-6 && (test_up - up).magnitude() > 1e-6 {
                    right = -right;
                    up = normalize_or_zero(normal.cross(right));
                }

                self.plane.origin = centroid;
                self.plane.normal = normal;

                self.frame.right = right;
                self.frame.up = up;
                self.frame.normal = self.plane.normal;

                self.edit_uv.origin = self.plane.origin;
                self.edit_uv.right = self.frame.right;
                self.edit_uv.up = self.frame.up;
                self.edit_uv.scale = 1.0;
                return;
            } else {
                self.plane = Default::default();
                self.frame = Default::default();
                self.edit_uv = Default::default();
                return;
            }
        }
        self.plane = Default::default();
        self.frame = Default::default();
        self.edit_uv = Default::default();
    }

    /// Map a UV point on the surface plane to world space (w = 0 plane).
    pub fn uv_to_world(&self, uv: Vec2<f32>) -> Vec3<f32> {
        self.edit_uv.origin
            + self.edit_uv.right * uv.x * self.edit_uv.scale
            + self.edit_uv.up * uv.y * self.edit_uv.scale
    }

    /// Map a UVW point (UV on the surface, W along the surface normal) to world space.
    pub fn uvw_to_world(&self, uv: Vec2<f32>, w: f32) -> Vec3<f32> {
        self.uv_to_world(uv) + self.frame.normal * w
    }

    pub fn world_to_uv(&self, p: Vec3<f32>) -> Vec2<f32> {
        let rel = p - self.edit_uv.origin;
        Vec2::new(rel.dot(self.edit_uv.right), rel.dot(self.edit_uv.up)) / self.edit_uv.scale
    }

    /// Normalized surface normal.
    pub fn normal(&self) -> Vec3<f32> {
        let n = self.plane.normal;
        let m = n.magnitude();
        if m > 1e-6 {
            n / m
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        }
    }

    /// Triangulate the owning sector in this surface's local UV space and return world vertices, indices, and UVs.
    /// This treats the sector's 3D polygon as the base face of the surface; any vertical/tilted walls are handled correctly.
    pub fn triangulate(
        &self,
        sector: &Sector,
        map: &Map,
    ) -> Option<(Vec<[f32; 4]>, Vec<(usize, usize, usize)>, Vec<[f32; 2]>)> {
        // 1) Get ordered 3D polygon for the sector
        let points3 = sector.vertices_world(map)?;
        if points3.len() < 3 {
            return None;
        }

        // 2) Project to this surface's local UV space
        let verts_uv: Vec<[f32; 2]> = points3
            .iter()
            .map(|p| {
                let uv = self.world_to_uv(*p);
                [uv.x, uv.y]
            })
            .collect();

        // 3) Triangulate in 2D (UV) using earcut (no holes for now)
        let flattened: Vec<f64> = verts_uv
            .iter()
            .flat_map(|v| [v[0] as f64, v[1] as f64])
            .collect();
        let holes: Vec<usize> = Vec::new();
        let idx = earcut(&flattened, &holes, 2).ok()?; // Vec<usize>

        // Convert to triangle triplets, flipping winding to match your renderer if needed
        let indices: Vec<(usize, usize, usize)> =
            idx.chunks_exact(3).map(|c| (c[2], c[1], c[0])).collect();

        // 4) Map UV back to world using this surface's frame
        let world_vertices: Vec<[f32; 4]> = verts_uv
            .iter()
            .map(|v| {
                let p = self.uv_to_world(vek::Vec2::new(v[0], v[1]));
                [p.x, p.y, p.z, 1.0]
            })
            .collect();

        Some((world_vertices, indices, verts_uv))
    }
}

fn normalize_or_zero(v: Vec3<f32>) -> Vec3<f32> {
    let m = v.magnitude();
    if m > 1e-6 { v / m } else { Vec3::zero() }
}

fn newell_plane(points: &[Vec3<f32>]) -> (Vec3<f32>, Vec3<f32>) {
    let mut centroid = Vec3::zero();
    let mut normal = Vec3::zero();
    let n = points.len();
    for i in 0..n {
        let current = points[i];
        let next = points[(i + 1) % n];
        centroid += current;
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    centroid /= n as f32;
    let m = normal.magnitude();
    if m > 1e-6 {
        normal /= m;
    } else {
        normal = Vec3::zero();
    }
    (centroid, normal)
}

fn stable_right(points: &[Vec3<f32>], normal: Vec3<f32>) -> Vec3<f32> {
    let n = points.len();
    let mut max_len = 0.0;
    let mut right = Vec3::zero();
    for i in 0..n {
        let edge = points[(i + 1) % n] - points[i];
        let proj = edge - normal * normal.dot(edge);
        let len = proj.magnitude();
        if len > max_len {
            max_len = len;
            right = proj;
        }
    }
    if max_len < 1e-6 {
        // fallback: pick any axis orthogonal to normal
        if normal.x.abs() < normal.y.abs() && normal.x.abs() < normal.z.abs() {
            right = Vec3::new(0.0, -normal.z, normal.y);
        } else if normal.y.abs() < normal.z.abs() {
            right = Vec3::new(-normal.z, 0.0, normal.x);
        } else {
            right = Vec3::new(-normal.y, normal.x, 0.0);
        }
    }
    normalize_or_zero(right)
}
