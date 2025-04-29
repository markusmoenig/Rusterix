use theframework::prelude::*;
use vek::Vec2;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub min: Vec2<f32>, // Bottom-left corner
    pub max: Vec2<f32>, // Top-right corner
}

impl BBox {
    /// Constructs a BBox from min and max coordinates
    pub fn new(min: Vec2<f32>, max: Vec2<f32>) -> Self {
        Self { min, max }
    }

    /// Constructs a BBox from position and size
    pub fn from_pos_size(pos: Vec2<f32>, size: Vec2<f32>) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }

    /// Returns the width and height of the bounding box
    pub fn size(&self) -> Vec2<f32> {
        self.max - self.min
    }

    /// Returns the center of the bounding box
    pub fn center(&self) -> Vec2<f32> {
        (self.min + self.max) * 0.5
    }

    /// Checks if a point is inside the bounding box
    pub fn contains(&self, point: Vec2<f32>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Expands (or shrinks) the bounding box by a given amount
    pub fn expand(&mut self, amount: Vec2<f32>) {
        self.min -= amount * 0.5;
        self.max += amount * 0.5;
    }

    /// Expands the bounding box to include another bounding box
    pub fn expand_bbox(&mut self, other: BBox) {
        self.min.x = self.min.x.min(other.min.x);
        self.min.y = self.min.y.min(other.min.y);
        self.max.x = self.max.x.max(other.max.x);
        self.max.y = self.max.y.max(other.max.y);
    }
}
