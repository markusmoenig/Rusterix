use rustc_hash::FxHashSet;
use theframework::prelude::*;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Item {
    /// The unique ID of the item
    pub id: u32,

    /// Maps the item to a creator ID
    pub creator_id: Uuid,

    /// The item's type or identifier (e.g., "Potion", "Sword")
    pub item_type: String,

    /// Maximum capacity of this container/stack (e.g., max stack size for stackable items)
    pub max_capacity: u32,

    /// Container: Holds nested items if this item can act as a container
    pub container: Option<Vec<Item>>,

    /// Attributes: Dynamic properties of the item
    pub attributes: ValueContainer,

    /// Dirty flags for static attributes
    pub dirty_flags: u8,

    /// Dirty dynamic attributes
    pub dirty_attributes: FxHashSet<String>,
}

impl Default for Item {
    fn default() -> Self {
        Self::new()
    }
}

impl Item {
    pub fn new() -> Self {
        Self {
            id: 0,
            creator_id: Uuid::new_v4(),
            item_type: String::new(),
            max_capacity: 1, // Default to 1 for non-stackable, non-container items
            container: None,
            attributes: ValueContainer::default(),
            dirty_flags: 0,
            dirty_attributes: FxHashSet::default(),
        }
    }

    /// Check if the item is a container or stackable
    pub fn is_container(&self) -> bool {
        self.container.is_some()
    }

    /// Check if there's space in the container
    pub fn has_space(&self) -> bool {
        if let Some(container) = &self.container {
            container.len() < self.max_capacity as usize
        } else {
            false
        }
    }

    /// Add an item to the container
    pub fn add_to_container(&mut self, item: Item) -> Result<(), String> {
        if let Some(container) = self.container.as_mut() {
            if container.len() < self.max_capacity as usize {
                container.push(item);
                self.mark_dirty_field(0b0001);
                Ok(())
            } else {
                Err("Container is full.".to_string())
            }
        } else {
            Err("This item is not a container.".to_string())
        }
    }

    /// Remove an item from the container by ID
    pub fn remove_from_container(&mut self, item_id: u32) -> Result<Item, String> {
        self.mark_dirty_field(0b0001);
        if let Some(container) = self.container.as_mut() {
            if let Some(index) = container.iter().position(|item| item.id == item_id) {
                return Ok(container.remove(index));
            }
            Err("Item not found in container.".to_string())
        } else {
            Err("This item is not a container.".to_string())
        }
    }

    /// Set a dynamic attribute and mark it as dirty
    pub fn set_attribute(&mut self, key: &str, value: Value) {
        self.attributes.set(key, value);
        self.mark_dirty_attribute(&key);
    }

    /// Get a dynamic attribute
    pub fn get_attribute(&self, key: &str) -> Option<&Value> {
        self.attributes.get(key)
    }

    /// Mark a static field as dirty
    fn mark_dirty_field(&mut self, field: u8) {
        self.dirty_flags |= field;
    }

    /// Mark a dynamic attribute as dirty
    fn mark_dirty_attribute(&mut self, key: &str) {
        self.dirty_attributes.insert(key.to_string());
    }

    /// Mark all fields and attributes as dirty
    pub fn mark_all_dirty(&mut self) {
        self.dirty_flags = 0b1111; // Mark all fields as dirty
        for key in self.attributes.keys() {
            self.dirty_attributes.insert(key.clone());
        }
        // Recursively mark all items in the container as dirty
        if let Some(container) = &mut self.container {
            for item in container.iter_mut() {
                item.mark_all_dirty();
            }
        }
    }

    /// Clear all dirty flags and attributes
    pub fn clear_dirty(&mut self) {
        self.dirty_flags = 0;
        self.dirty_attributes.clear();
        // Recursively clear dirty flags for all items in the container
        if let Some(container) = &mut self.container {
            for item in container.iter_mut() {
                item.clear_dirty();
            }
        }
    }

    /// Check if the item is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty_flags != 0
            || !self.dirty_attributes.is_empty()
            || self
                .container
                .as_ref()
                .map_or(false, |c| c.iter().any(|item| item.is_dirty()))
    }

    /// Generate an `ItemUpdate` containing only dirty fields and attributes
    pub fn get_update(&self) -> ItemUpdate {
        let mut updated_attributes = FxHashMap::default();
        for key in &self.dirty_attributes {
            if let Some(value) = self.attributes.get(key) {
                updated_attributes.insert(key.clone(), value.clone());
            }
        }

        let container_updates = if let Some(container) = &self.container {
            Some(
                container
                    .iter()
                    .filter(|item| item.is_dirty())
                    .map(|item| item.get_update())
                    .collect(),
            )
        } else {
            None
        };

        ItemUpdate {
            id: self.id,
            item_type: if self.dirty_flags & 0b0001 != 0 {
                Some(self.item_type.clone())
            } else {
                None
            },
            max_capacity: if self.dirty_flags & 0b0010 != 0 {
                Some(self.max_capacity)
            } else {
                None
            },
            attributes: updated_attributes,
            container_updates,
        }
    }

    /// Apply an `ItemUpdate` to this item
    pub fn apply_update(&mut self, update: ItemUpdate) {
        // Validate ID matches
        if self.id != update.id {
            eprintln!("Update ID does not match Item ID!");
            return;
        }

        // Update static fields
        if let Some(new_item_type) = update.item_type {
            self.item_type = new_item_type;
            self.dirty_flags |= 0b0001;
        }
        if let Some(new_max_capacity) = update.max_capacity {
            self.max_capacity = new_max_capacity;
            self.dirty_flags |= 0b0010;
        }

        // Update dynamic attributes
        for (key, value) in update.attributes {
            self.attributes.set(&key, value.clone());
            self.dirty_attributes.insert(key);
        }

        // Recursively apply updates to items in the container
        if let Some(container_updates) = update.container_updates {
            if let Some(container) = &mut self.container {
                for update in container_updates {
                    if let Some(item) = container.iter_mut().find(|item| item.id == update.id) {
                        item.apply_update(update);
                    }
                }
            }
        }
    }
}

/// Represents a partial update for an `Item`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemUpdate {
    pub id: u32,
    pub item_type: Option<String>,
    pub max_capacity: Option<u32>,
    pub attributes: FxHashMap<String, Value>,
    pub container_updates: Option<Vec<ItemUpdate>>,
}

impl ItemUpdate {
    /// Serialize (pack) an `ItemUpdate` into a `Vec<u8>` using bincode, discarding errors
    pub fn pack(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_else(|_| Vec::new())
    }

    /// Deserialize (unpack) a `Vec<u8>` into an `ItemUpdate` using bincode, discarding errors
    pub fn unpack(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap_or_else(|_| Self {
            id: 0,
            item_type: None,
            max_capacity: None,
            attributes: FxHashMap::default(),
            container_updates: None,
        })
    }
}
