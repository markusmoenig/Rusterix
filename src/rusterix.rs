use crate::prelude::*;
use rustc_hash::FxHashMap;

/// Rusterix can server as a server or client or both for solo games.
pub struct Rusterix {
    pub assets: Assets,
    pub server: Server,
}

impl Default for Rusterix {
    fn default() -> Self {
        Self::new()
    }
}

impl Rusterix {
    pub fn new() -> Self {
        Self {
            assets: Assets::default(),
            server: Server::default(),
        }
    }

    /// Set the assets
    pub fn set_assets(&mut self, assets: Assets) {
        self.assets = assets
    }

    /// Create the server regions.
    pub fn create_regions(&mut self) {
        for (name, map) in &self.assets.maps {
            self.server
                .create_region(name.clone(), map.clone(), &self.assets.entities);
        }
    }

    /// Update existing entities or create new ones
    pub fn process_entity_updates(&self, entities: &mut Vec<Entity>, updates: Vec<EntityUpdate>) {
        // Create a mapping from entity ID to index for efficient lookup
        let mut entity_map: FxHashMap<u32, usize> = entities
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.id, index))
            .collect();

        for update in updates {
            if let Some(&index) = entity_map.get(&update.id) {
                // Entity exists, apply the update
                entities[index].apply_update(update);
            } else {
                // Entity does not exist, create a new one
                let mut new_entity = Entity {
                    id: update.id,
                    ..Default::default()
                };
                new_entity.apply_update(update);

                // Add to the entity list
                let new_entity_id = new_entity.id; // Copy or borrow the ID
                entities.push(new_entity);

                // Update the map with the new entity's ID
                entity_map.insert(new_entity_id, entities.len() - 1);
            }
        }
    }
}
