use rustc_hash::FxHashSet;
use theframework::prelude::*;
use vek::{Vec2, Vec3};

use crate::{prelude::*, EntityAction};

/// The Rust representation of an Entity. The real entity class lives in Python, this class is the Rust side
/// instantiation (to avoid unnecessary Python look ups for common attributes). The class gets synced with the Python side.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity {
    /// The id of the entity in the entity manager
    pub id: u32,

    /// Maps the entity to a creator id
    pub creator_id: Uuid,

    /// The XZ orientation
    pub orientation: Vec2<f32>,
    /// The position in the map
    pub position: Vec3<f32>,
    /// The vertical camera tilt, 0.0 means flat, no tilt.
    pub tilt: f32,

    /// The current action
    #[serde(skip)]
    pub action: EntityAction,

    /// Attributes
    pub attributes: ValueContainer,

    /// Dirty static atrributes
    pub dirty_flags: u8,

    /// Dirty Attributes
    pub dirty_attributes: FxHashSet<String>,
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity {
    pub fn new() -> Self {
        Self {
            id: 0,
            creator_id: Uuid::new_v4(),

            orientation: Vec2::new(1.0, 0.0),
            position: Vec3::new(0.0, 1.0, 0.0),
            tilt: 0.0,

            action: EntityAction::Off,

            attributes: ValueContainer::default(),

            dirty_flags: 0,
            dirty_attributes: FxHashSet::default(),
        }
    }

    /// Get the XZ position.
    pub fn get_pos_xz(&self) -> Vec2<f32> {
        Vec2::new(self.position.x, self.position.z)
    }

    /// Computes the look-at target based on position, orientation, and vertical tilt (tilt).
    pub fn camera_look_at(&self) -> Vec3<f32> {
        let vertical_offset = self.orientation.magnitude() * self.tilt.sin();
        Vec3::new(
            self.position.x + self.orientation.x,
            self.position.y + vertical_offset,
            self.position.z + self.orientation.y,
        )
    }

    /// Rotates the entity to the left by a certain degree.
    pub fn turn_left(&mut self, degrees: f32) {
        self.rotate_orientation(-degrees.to_radians());
    }

    /// Rotates the entity to the right by a certain degree.
    pub fn turn_right(&mut self, degrees: f32) {
        self.rotate_orientation(degrees.to_radians());
    }

    /// Moves the entity forward along its current orientation.
    pub fn move_forward(&mut self, distance: f32) {
        self.position.x += self.orientation.x * distance;
        self.position.z += self.orientation.y * distance;
        self.mark_dirty_field(0b0001);
    }

    /// Moves the entity backward along its current orientation.
    pub fn move_backward(&mut self, distance: f32) {
        self.position.x -= self.orientation.x * distance;
        self.position.z -= self.orientation.y * distance;
        self.mark_dirty_field(0b0001);
    }

    /// Helper method to rotate the orientation vector by a given angle in radians.
    fn rotate_orientation(&mut self, radians: f32) {
        let cos_angle = radians.cos();
        let sin_angle = radians.sin();
        let new_x = self.orientation.x * cos_angle - self.orientation.y * sin_angle;
        let new_y = self.orientation.x * sin_angle + self.orientation.y * cos_angle;
        self.orientation = Vec2::new(new_x, new_y).normalized();
        self.mark_dirty_field(0b0010);
    }

    /// Applies the camera's position and look-at parameters based on the entity's state.
    pub fn apply_to_camera(&self, camera: &mut Box<dyn D3Camera>) {
        // println!("{} {}", self.position, self.orientation);
        let id = camera.id();

        if id != "iso" {
            camera.set_parameter_vec3("position", self.position);
            camera.set_parameter_vec3("center", self.camera_look_at());
        } else {
            let p = Vec3::new(self.position.x, 0.0, self.position.z);
            camera.set_parameter_vec3("center", p);
            camera.set_parameter_vec3("position", p + vek::Vec3::new(-10.0, 10.0, 10.0));
        }
    }

    /// Set the position and mark it as dirty
    pub fn set_position(&mut self, new_position: Vec3<f32>) {
        if self.position != new_position {
            self.position = new_position;
            self.mark_dirty_field(0b0001);
        }
    }

    /// Set the orientation and mark it as dirty
    pub fn set_orientation(&mut self, new_orientation: Vec2<f32>) {
        if self.orientation != new_orientation {
            self.orientation = new_orientation;
            self.mark_dirty_field(0b0010);
        }
    }

    /// Set the tilt and mark it as dirty
    pub fn set_tilt(&mut self, new_tilt: f32) {
        if self.tilt != new_tilt {
            self.tilt = new_tilt;
            self.mark_dirty_field(0b0100);
        }
    }

    /// Maps a normalized screen coordinate (0.0 to 1.0) to a `tilt` angle.
    /// `0.0` -> maximum downward tilt, `1.0` -> maximum upward tilt.
    pub fn set_tilt_from_screen_coordinate(&mut self, screen_y: f32) {
        // Map the normalized screen coordinate to a range of angles (e.g., -π/4 to π/4)
        let max_tilt = std::f32::consts::FRAC_PI_4; // 45 degrees
        self.tilt = (screen_y - 0.5) * 2.0 * max_tilt;
        self.mark_dirty_field(0b0100);
    }

    /// Set a dynamic attribute and mark it as dirty
    pub fn set_attribute(&mut self, key: String, value: Value) {
        self.attributes.set(&key, value);
        self.mark_dirty_attribute(&key);
    }

    /// Get the given String
    pub fn get_attr_string(&self, key: &str) -> Option<String> {
        self.attributes.get(key).map(|value| value.to_string())
    }

    /// Get the given Uuid
    pub fn get_attr_uuid(&self, key: &str) -> Option<Uuid> {
        if let Some(Value::Id(value)) = self.attributes.get(key) {
            Some(*value)
        } else {
            None
        }
    }

    /// Returns true if this entity is a player
    pub fn is_player(&self) -> bool {
        if let Some(Value::Bool(value)) = self.attributes.get("is_player") {
            *value
        } else {
            false
        }
    }

    /// Mark a static field as dirty
    fn mark_dirty_field(&mut self, field: u8) {
        self.dirty_flags |= field;
    }

    /// Mark a dynamic attribute as dirty
    fn mark_dirty_attribute(&mut self, key: &str) {
        self.dirty_attributes.insert(key.to_string());
    }

    /// Mark all fields and attributes as dirty.
    pub fn mark_all_dirty(&mut self) {
        self.dirty_flags = 0b0111;
        self.dirty_attributes = self.attributes.keys().cloned().collect();
    }

    /// Check if the entity is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty_flags != 0 || !self.dirty_attributes.is_empty()
    }

    /// Mark all fields and attributes as dirty
    pub fn set_all_dirty(&mut self) {
        self.dirty_flags = 0b0111;
        self.dirty_attributes = self.attributes.keys().cloned().collect();
    }

    /// Mark all static fields as dirty
    pub fn set_static_dirty(&mut self) {
        self.dirty_flags = 0b0111;
        self.dirty_attributes.clear();
    }

    /// Clear all dirty flags and attributes
    pub fn clear_dirty(&mut self) {
        self.dirty_flags = 0;
        self.dirty_attributes.clear();
    }

    /// Get a partial update containing only dirty fields and attributes
    pub fn get_update(&self) -> EntityUpdate {
        let mut updated_attributes = FxHashMap::default();
        for key in &self.dirty_attributes {
            if let Some(value) = self.attributes.get(key) {
                updated_attributes.insert(key.clone(), value.clone());
            }
        }

        EntityUpdate {
            id: self.id,
            position: if self.dirty_flags & 0b0001 != 0 {
                Some(self.position)
            } else {
                None
            },
            orientation: if self.dirty_flags & 0b0010 != 0 {
                Some(self.orientation)
            } else {
                None
            },
            tilt: if self.dirty_flags & 0b0100 != 0 {
                Some(self.tilt)
            } else {
                None
            },
            attributes: updated_attributes,
        }
    }

    /// Apply an update to the entity
    pub fn apply_update(&mut self, update: EntityUpdate) {
        // Validate ID matches
        if self.id != update.id {
            eprintln!("Update ID does not match Entity ID!");
            return;
        }

        // Update static fields
        if let Some(new_position) = update.position {
            self.position = new_position;
            self.mark_dirty_field(0b0001);
        }
        if let Some(new_orientation) = update.orientation {
            self.orientation = new_orientation;
            self.mark_dirty_field(0b0010);
        }
        if let Some(new_camera_tilt) = update.tilt {
            self.tilt = new_camera_tilt;
            self.mark_dirty_field(0b0100);
        }

        // Update dynamic attributes
        for (key, value) in update.attributes {
            self.attributes.set(&key, value.clone());
            self.mark_dirty_attribute(&key);
        }
    }

    /// Sets the orientation to face east.
    pub fn face_east(&mut self) {
        self.set_orientation(Vec2::new(1.0, 0.0));
    }

    /// Sets the orientation to face west.
    pub fn face_west(&mut self) {
        self.set_orientation(Vec2::new(-1.0, 0.0));
    }

    /// Sets the orientation to face north.
    pub fn face_north(&mut self) {
        self.set_orientation(Vec2::new(0.0, -1.0));
    }

    /// Sets the orientation to face south.
    pub fn face_south(&mut self) {
        self.set_orientation(Vec2::new(0.0, 1.0));
    }

    /// Sets the orientation to face a specific point.
    pub fn face_at(&mut self, target: Vec2<f32>) {
        let current_position = self.get_pos_xz();
        let direction = (target - current_position).normalized();
        self.set_orientation(direction);
    }
}

// EntityUpdate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityUpdate {
    pub id: u32,
    pub position: Option<Vec3<f32>>,
    pub orientation: Option<Vec2<f32>>,
    pub tilt: Option<f32>,
    pub attributes: FxHashMap<String, Value>,
}

impl EntityUpdate {
    /// Serialize (pack) an `EntityUpdate` into a `Vec<u8>` using bincode, discarding errors
    pub fn pack(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_else(|_| Vec::new())
    }

    /// Deserialize (unpack) a `Vec<u8>` into an `EntityUpdate` using bincode, discarding errors
    pub fn unpack(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap_or_else(|_| Self {
            id: 0,
            position: None,
            orientation: None,
            tilt: None,
            attributes: FxHashMap::default(),
        })
    }
}
