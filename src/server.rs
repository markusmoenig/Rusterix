pub mod assets;
pub mod entity;
pub mod message;
pub mod region;

use crossbeam_channel::{Receiver, Sender};

use crate::prelude::*;
use std::{
    // borrow::BorrowMut,
    sync::{Arc, LazyLock, RwLock},
};
use theframework::prelude::*;

// Pipes to the regions
type RegionRegistry = Arc<RwLock<FxHashMap<u32, Sender<RegionMessage>>>>;
static REGIONPIPE: LazyLock<RegionRegistry> =
    LazyLock::new(|| Arc::new(RwLock::new(FxHashMap::default())));

// List of currently active local players
type Player = Arc<RwLock<Vec<(u32, u32)>>>;
static LOCAL_PLAYERS: LazyLock<Player> = LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// Send from a player script to register the player.
fn register_player(region_id: u32, entity_id: u32) {
    if let Ok(pipes) = REGIONPIPE.read() {
        if let Some(sender) = pipes.get(&region_id) {
            sender
                .send(RegionMessage::RegisterPlayer(entity_id))
                .unwrap();
        }
    }
    if let Ok(mut players) = LOCAL_PLAYERS.write() {
        players.push((region_id, entity_id));
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ServerState {
    Off,
    Running,
    Paused,
}

pub struct Server {
    pub id_gen: u32,

    pub region_id_map: FxHashMap<Uuid, u32>,
    from_region: Vec<Receiver<RegionMessage>>,

    pub entities: FxHashMap<u32, Vec<Entity>>,

    pub state: ServerState,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            id_gen: 0,

            region_id_map: FxHashMap::default(),
            from_region: vec![],

            entities: FxHashMap::default(),

            state: ServerState::Off,
        }
    }

    /// Set the server state.
    pub fn set_state(&mut self, state: ServerState) {
        self.state = state;
    }

    /// Create the given region instance.
    pub fn create_region_instance(
        &mut self,
        name: String,
        map: Map,
        entities: &FxHashMap<String, String>,
    ) {
        let mut region_instance = RegionInstance::default();
        region_instance.id = self.get_next_id();

        self.region_id_map.insert(map.id, region_instance.id);

        if let Ok(mut pipes) = REGIONPIPE.write() {
            pipes.insert(region_instance.id, region_instance.to_sender.clone());
        }

        self.from_region.push(region_instance.from_receiver.clone());

        region_instance.init(name, map, entities);
        region_instance.run();
    }

    /// Get entities for a given region.
    pub fn get_entities(&self, region_id: &Uuid) -> Option<&Vec<Entity>> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            self.entities.get(region_id)
        } else {
            None
        }
    }

    /// Updates the entities from the regions.
    pub fn update_entities(&mut self) {
        for receiver in &self.from_region {
            while let Ok(message) = receiver.try_recv() {
                #[allow(clippy::single_match)]
                match message {
                    RegionMessage::EntitiesUpdate(id, serialized_updates) => {
                        let updates: Vec<EntityUpdate> = serialized_updates
                            .into_iter()
                            .map(|data| EntityUpdate::unpack(&data))
                            .collect();

                        if let Some(entities) = self.entities.get_mut(&id) {
                            Self::process_entity_updates(entities, updates);
                        } else {
                            let mut entities = vec![];
                            Self::process_entity_updates(&mut entities, updates);
                            self.entities.insert(id, entities);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Update existing entities (or create new ones if they do not exist).
    pub fn process_entity_updates(entities: &mut Vec<Entity>, updates: Vec<EntityUpdate>) {
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

    /// Send a local player event to the registered players
    pub fn local_player_event(&mut self, event: String, value: Value) {
        if let Ok(local_players) = LOCAL_PLAYERS.read() {
            if let Ok(pipe) = REGIONPIPE.read() {
                for (region_id, entity_id) in local_players.iter() {
                    if let Some(sender) = pipe.get(region_id) {
                        match sender.send(RegionMessage::UserEvent(
                            *entity_id,
                            event.clone(),
                            value.clone(),
                        )) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("{:?}", err.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Shuts down all region instances.
    pub fn stop(&mut self) {
        if let Ok(pipes) = REGIONPIPE.read() {
            for sender in pipes.values() {
                sender.send(RegionMessage::Quit).unwrap();
            }
        }
        if let Ok(mut pipes) = REGIONPIPE.write() {
            pipes.clear();
        }
        if let Ok(mut players) = LOCAL_PLAYERS.write() {
            players.clear();
        }
        self.id_gen = 0;
        self.region_id_map.clear();
        self.state = ServerState::Off;
    }

    /// Create a id
    pub fn get_next_id(&mut self) -> u32 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }
}
