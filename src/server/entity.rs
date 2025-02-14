use rand::Rng;
use rustc_hash::FxHashSet;
use std::collections::VecDeque;
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

    /// Dirty static attributes
    /// The `dirty_flags` field is a bitmask representing changes to various components of the entity.
    /// Each bit corresponds to a specific type of change:
    /// - `0b00000001` (1): Position changed
    /// - `0b00000010` (2): Orientation changed
    /// - `0b00000100` (4): Tilt changed
    /// - `0b00001000` (8): Inventory changed
    /// - `0b00010000` (16): Equipped items changed
    /// - `0b00100000` (32): Wallet changed
    pub dirty_flags: u8,

    /// Dirty Attributes
    pub dirty_attributes: FxHashSet<String>,

    /// Inventory: A container for the entity's items
    pub inventory: FxHashMap<u32, Item>,

    /// Track added items
    pub inventory_additions: FxHashMap<u32, Item>,
    /// Track removed items
    pub inventory_removals: FxHashSet<u32>,
    /// Track updated items
    pub inventory_updates: FxHashMap<u32, ItemUpdate>,

    /// Equipped items: Slot to item ID mapping
    pub equipped: FxHashMap<String, u32>, // "main_hand" -> Item ID

    /// Wallet
    pub wallet: Wallet,
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

            inventory: FxHashMap::default(),
            inventory_additions: FxHashMap::default(),
            inventory_removals: FxHashSet::default(),
            inventory_updates: FxHashMap::default(),

            equipped: FxHashMap::default(),

            wallet: Wallet::default(),
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

    /// Set the position as a Vec2 and mark it as dirty
    pub fn set_pos_xz(&mut self, new_position: Vec2<f32>) {
        self.position.x = new_position.x;
        self.position.z = new_position.y;
        self.mark_dirty_field(0b0001);
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

    /// Add an item to the entity's inventory and track additions
    pub fn add_item(&mut self, item: Item) {
        self.inventory.insert(item.id, item.clone());
        self.inventory_removals.remove(&item.id); // If it was previously removed, undo that
        self.inventory_additions.insert(item.id, item);
        self.mark_dirty_field(0b1000);
    }

    /// Remove an item from the entity's inventory and track removals
    pub fn remove_item(&mut self, item_id: u32) -> Option<Item> {
        if let Some(item) = self.inventory.remove(&item_id) {
            self.inventory_removals.insert(item_id);
            self.inventory_additions.remove(&item_id); // If it was previously added, undo that
            self.mark_dirty_field(0b1000);
            Some(item)
        } else {
            None
        }
    }

    /// Get a reference to an item by ID
    pub fn get_item(&self, item_id: u32) -> Option<&Item> {
        self.inventory.get(&item_id)
    }

    /// Get a mutable reference to an item by ID
    pub fn get_item_mut(&mut self, item_id: u32) -> Option<&mut Item> {
        if let Some(item) = self.inventory.get_mut(&item_id) {
            // Mark the item as updated
            self.inventory_updates.insert(item_id, item.get_update());
            Some(item)
        } else {
            None
        }
    }

    /// Equip an item in a specific slot
    pub fn equip_item(&mut self, item_id: u32, slot: &str) -> Result<(), String> {
        // Check if the item exists in the inventory
        if self.inventory.contains_key(&item_id) {
            self.equipped.insert(slot.to_string(), item_id);
            self.dirty_flags |= 0b10000; // Mark equipped slots as dirty
            Ok(())
        } else {
            Err("Item not found in inventory.".to_string())
        }
    }

    /// Unequip an item from a specific slot
    pub fn unequip_item(&mut self, slot: &str) -> Result<(), String> {
        if self.equipped.remove(slot).is_some() {
            self.dirty_flags |= 0b10000; // Mark equipped slots as dirty
            Ok(())
        } else {
            Err("No item equipped in the given slot.".to_string())
        }
    }

    /// Get the item ID equipped in a specific slot
    pub fn get_equipped_item(&self, slot: &str) -> Option<u32> {
        self.equipped.get(slot).copied()
    }

    /// Add the given currency to the wallet.
    pub fn add_currency(
        &mut self,
        symbol: &str,
        amount: i64,
        currencies: &Currencies,
    ) -> Result<(), String> {
        self.wallet.add(symbol, amount, currencies)?;
        self.dirty_flags |= 0b100000;
        Ok(())
    }

    /// Spend the given currency.
    pub fn spend_currency(
        &mut self,
        base_amount: i64,
        currencies: &Currencies,
    ) -> Result<(), String> {
        self.wallet.spend(base_amount, currencies)?;
        self.dirty_flags |= 0b100000;
        Ok(())
    }

    /// Set a dynamic attribute and mark it as dirty
    pub fn set_attribute(&mut self, key: &str, value: Value) {
        self.attributes.set(key, value);
        self.mark_dirty_attribute(key);
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
        self.dirty_flags = 0b11111;
        self.dirty_attributes = self.attributes.keys().cloned().collect();
    }

    /// Check if the entity is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty_flags != 0 || !self.dirty_attributes.is_empty()
    }

    /// Mark all static fields as dirty
    pub fn set_static_dirty(&mut self) {
        self.dirty_flags = 0b11111;
        self.dirty_attributes.clear();
    }

    /// Clear all dirty flags and attributes
    pub fn clear_dirty(&mut self) {
        self.dirty_flags = 0;
        self.dirty_attributes.clear();
        self.inventory_additions.clear();
        self.inventory_removals.clear();
        self.inventory_updates.clear();
    }

    /// Get a partial update containing only dirty fields and attributes
    pub fn get_update(&self) -> EntityUpdate {
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
            attributes: self
                .dirty_attributes
                .iter()
                .filter_map(|key| self.attributes.get(key).map(|v| (key.clone(), v.clone())))
                .collect(),
            inventory_additions: if !self.inventory_additions.is_empty() {
                Some(self.inventory_additions.clone())
            } else {
                None
            },
            inventory_removals: if !self.inventory_removals.is_empty() {
                Some(self.inventory_removals.clone())
            } else {
                None
            },
            inventory_updates: if !self.inventory_updates.is_empty() {
                Some(self.inventory_updates.clone())
            } else {
                None
            },
            equipped_updates: if self.dirty_flags & 0b10000 != 0 {
                Some(self.equipped.clone())
            } else {
                None
            },
            wallet_updates: if self.dirty_flags & 0b100000 != 0 {
                Some(self.wallet.balances.clone())
            } else {
                None
            },
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
        }
        if let Some(new_orientation) = update.orientation {
            self.orientation = new_orientation;
        }
        if let Some(new_camera_tilt) = update.tilt {
            self.tilt = new_camera_tilt;
        }

        // Update dynamic attributes
        for (key, value) in update.attributes {
            self.attributes.set(&key, value.clone());
            self.mark_dirty_attribute(&key);
        }

        // Apply inventory additions
        if let Some(inventory_additions) = update.inventory_additions {
            for (item_id, item) in inventory_additions {
                self.inventory.insert(item_id, item);
            }
        }

        // Apply inventory removals
        if let Some(inventory_removals) = update.inventory_removals {
            for item_id in inventory_removals {
                self.inventory.remove(&item_id);
            }
        }

        // Apply inventory updates
        if let Some(inventory_updates) = update.inventory_updates {
            for (item_id, item_update) in inventory_updates {
                if let Some(item) = self.inventory.get_mut(&item_id) {
                    item.apply_update(item_update);
                }
            }
        }

        // Apply equipped slot updates
        if let Some(equipped_updates) = update.equipped_updates {
            self.equipped = equipped_updates;
        }

        // Apply wallet updates
        if let Some(wallet_updates) = update.wallet_updates {
            for (symbol, balance) in wallet_updates {
                self.wallet.balances.insert(symbol, balance);
            }
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
        let delta = target - current_position;
        if delta.magnitude_squared() < f32::EPSILON {
            return; // Don't face if target is the same as current
        }
        let direction = delta.normalized();
        self.set_orientation(direction);
    }

    /// Sets the orientation to face a random direction.
    pub fn face_random(&mut self) {
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::TAU); // TAU is 2π
        let direction = Vec2::new(angle.cos(), angle.sin());
        self.set_orientation(direction);
    }

    /// Create an iterator over the inventory.
    pub fn iter_inventory(&self) -> InventoryIterator {
        InventoryIterator::new(self)
    }

    /// Create a mutable iterator over the inventory.
    pub fn iter_inventory_mut(&mut self) -> InventoryIteratorMut {
        InventoryIteratorMut::new(self)
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
    pub inventory_additions: Option<FxHashMap<u32, Item>>,
    pub inventory_removals: Option<FxHashSet<u32>>,
    pub inventory_updates: Option<FxHashMap<u32, ItemUpdate>>,
    pub equipped_updates: Option<FxHashMap<String, u32>>,
    pub wallet_updates: Option<FxHashMap<String, i64>>,
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
            inventory_updates: None,
            inventory_additions: None,
            inventory_removals: None,
            equipped_updates: None,
            wallet_updates: None,
        })
    }
}

/// Iterator over inventory
pub struct InventoryIterator<'a> {
    stack: VecDeque<Box<dyn Iterator<Item = &'a Item> + 'a>>,
}

impl<'a> InventoryIterator<'a> {
    pub fn new(entity: &'a Entity) -> Self {
        let iter: Box<dyn Iterator<Item = &'a Item> + 'a> = Box::new(entity.inventory.values());
        let mut stack = VecDeque::new();
        stack.push_back(iter); // Push the boxed iterator
        InventoryIterator { stack }
    }
}

impl<'a> Iterator for InventoryIterator<'a> {
    type Item = &'a Item;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(iter) = self.stack.back_mut() {
            if let Some(item) = iter.next() {
                if let Some(container) = &item.container {
                    let child_iter: Box<dyn Iterator<Item = &'a Item> + 'a> =
                        Box::new(container.iter());
                    self.stack.push_back(child_iter);
                }
                return Some(item);
            } else {
                self.stack.pop_back();
            }
        }
        None
    }
}

/// Mut iterator over inventory
pub struct InventoryIteratorMut<'a> {
    stack: VecDeque<Box<dyn Iterator<Item = &'a mut Item> + 'a>>,
}

impl<'a> InventoryIteratorMut<'a> {
    pub fn new(entity: &'a mut Entity) -> Self {
        let iter: Box<dyn Iterator<Item = &'a mut Item> + 'a> =
            Box::new(entity.inventory.values_mut());
        let mut stack = VecDeque::new();
        stack.push_back(iter);
        InventoryIteratorMut { stack }
    }
}

impl<'a> Iterator for InventoryIteratorMut<'a> {
    type Item = &'a mut Item;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(iter) = self.stack.back_mut() {
            if let Some(item) = iter.next() {
                // Use a raw pointer to bypass the borrow checker
                let container_ptr = item.container.as_mut().map(|c| c as *mut Vec<Item>);

                if let Some(ptr) = container_ptr {
                    let container = unsafe { &mut *ptr };
                    let child_iter: Box<dyn Iterator<Item = &'a mut Item> + 'a> =
                        Box::new(container.iter_mut());
                    self.stack.push_back(child_iter);
                }

                return Some(item);
            } else {
                self.stack.pop_back();
            }
        }
        None
    }
}
