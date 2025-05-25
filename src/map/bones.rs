use serde::{Deserialize, Serialize};
use vek::{Mat4, Vec2};

#[derive(Serialize, Deserialize, Clone, Debug)]
/// A single bone in the skeleton, representing a joint or limb
pub struct Bone {
    /// Unique bone identifier
    pub id: u32,

    /// Human-readable name (e.g. "UpperArm.L")
    pub name: String,

    /// Optional parent bone ID for hierarchy
    pub parent: Option<u32>,

    /// Reference to a linedef in 2D rigging (optional)
    pub linedef_id: Option<u32>,

    /// Rest pose transform as a full 3D matrix (bind pose)
    pub rest_matrix: Mat4<f32>,
}

impl Bone {
    /// Extract 2D position from the rest matrix
    pub fn rest_position_2d(&self) -> Vec2<f32> {
        Vec2::new(self.rest_matrix.cols.w.x, self.rest_matrix.cols.w.y)
    }

    /// Extract 2D rotation (in radians) from the rest matrix
    pub fn rest_rotation_2d(&self) -> f32 {
        self.rest_matrix.cols.x.x.atan2(-self.rest_matrix.cols.x.y)
    }
}

/// A per-bone transform override for a given frame
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BonePose {
    /// ID of the bone this pose applies to
    pub bone_id: u32,

    /// Pose transform matrix (relative to rest pose)
    pub pose_matrix: Mat4<f32>,
}

impl BonePose {
    /// Extract 2D position from the pose matrix
    pub fn position_2d(&self) -> Vec2<f32> {
        Vec2::new(self.pose_matrix.cols.w.x, self.pose_matrix.cols.w.y)
    }

    /// Extract 2D rotation (in radians) from the pose matrix
    pub fn rotation_2d(&self) -> f32 {
        self.pose_matrix.cols.x.x.atan2(-self.pose_matrix.cols.x.y)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// A full animation frame (keyframe), holding transforms for all bones
pub struct Pose {
    pub frame_index: usize,
    pub bone_poses: Vec<BonePose>,
}

impl Default for Pose {
    fn default() -> Self {
        Pose::new()
    }
}

impl Pose {
    pub fn new() -> Self {
        Self {
            frame_index: 0,
            bone_poses: vec![],
        }
    }
}

/// A complete skeletal animation track
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SkeletalAnimation {
    /// Name of the animation (e.g. "Walk", "Idle")
    pub name: String,

    /// Frames per second for playback
    pub fps: i32,

    /// Bone definitions and hierarchy (bind pose)
    pub bones: Vec<Bone>,

    /// Animation keyframes
    pub keyframes: Vec<Pose>,
}

impl Default for SkeletalAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl SkeletalAnimation {
    pub fn new() -> Self {
        Self {
            name: "Unnamed".into(),
            fps: 0,
            bones: vec![],
            keyframes: vec![],
        }
    }
}
