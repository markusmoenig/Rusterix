pub mod assets;
pub mod currency;
pub mod entity;
pub mod item;
pub mod message;
pub mod py_fn;
pub mod region;

use crossbeam_channel::{Receiver, Sender};

use crate::prelude::*;
use std::sync::{Arc, LazyLock, RwLock};
use theframework::prelude::*;

// Pipes to the regions
type RegionRegistry = Arc<RwLock<FxHashMap<u32, Sender<RegionMessage>>>>;
static REGIONPIPE: LazyLock<RegionRegistry> =
    LazyLock::new(|| Arc::new(RwLock::new(FxHashMap::default())));

// List of currently active local players
type Player = Arc<RwLock<Vec<(u32, u32)>>>;
static LOCAL_PLAYERS: LazyLock<Player> = LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

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
    pub items: FxHashMap<u32, Vec<Item>>,

    pub times: FxHashMap<u32, TheTime>,

    pub state: ServerState,

    pub log: String,
    pub log_changed: bool,
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

            times: FxHashMap::default(),

            entities: FxHashMap::default(),
            items: FxHashMap::default(),

            state: ServerState::Off,

            log: String::new(),
            log_changed: true,
        }
    }

    /// Clear the log
    pub fn clear_log(&mut self) {
        self.log = String::new();
    }

    /// Retrieve the log
    pub fn get_log(&mut self) -> String {
        self.log_changed = false;
        self.log.clone()
    }

    /// Set the server state.
    pub fn set_state(&mut self, state: ServerState) {
        self.state = state;
    }

    /// Create the given region instance.
    pub fn create_region_instance(&mut self, name: String, map: Map, assets: &Assets) {
        let mut region_instance = RegionInstance::default();
        region_instance.id = self.get_next_id();

        self.region_id_map.insert(map.id, region_instance.id);

        if let Ok(mut pipes) = REGIONPIPE.write() {
            pipes.insert(region_instance.id, region_instance.to_sender.clone());
        }

        self.from_region.push(region_instance.from_receiver.clone());

        region_instance.init(name, map, assets);
        region_instance.run();
    }

    /// Get entities and items for a given region.
    pub fn get_entities_items(
        &self,
        region_id: &Uuid,
    ) -> (Option<&Vec<Entity>>, Option<&Vec<Item>>) {
        let mut rc: (Option<&Vec<Entity>>, Option<&Vec<Item>>) = (None, None);

        rc.0 = if let Some(region_id) = self.region_id_map.get(region_id) {
            self.entities.get(region_id)
        } else {
            None
        };

        rc.1 = if let Some(region_id) = self.region_id_map.get(region_id) {
            self.items.get(region_id)
        } else {
            None
        };

        rc
    }

    /// Get the current time for the given region.
    pub fn get_time(&self, region_id: &Uuid) -> Option<TheTime> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            if let Some(time) = self.times.get(region_id) {
                return Some(*time);
            }
        }
        None
    }

    /// Set the current time for the given region.
    pub fn set_time(&mut self, region_id: &Uuid, time: TheTime) -> TheTime {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            if let Ok(pipe) = REGIONPIPE.read() {
                if let Some(sender) = pipe.get(region_id) {
                    self.times.clear();
                    match sender.send(RegionMessage::Time(*region_id, time)) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("{:?}", err.to_string());
                        }
                    }
                }
            }
        }
        TheTime::default()
    }

    /// Retrieves all messages from the regions.
    pub fn update(&mut self) {
        for receiver in &self.from_region {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    RegionMessage::RegisterPlayer(region_id, entity_id) => {
                        if let Ok(mut players) = LOCAL_PLAYERS.write() {
                            println!("Registering player: {} {}", region_id, entity_id);
                            players.push((region_id, entity_id));
                        }
                    }
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
                    RegionMessage::ItemsUpdate(id, serialized_updates) => {
                        let updates: Vec<ItemUpdate> = serialized_updates
                            .into_iter()
                            .map(|data| ItemUpdate::unpack(&data))
                            .collect();

                        if let Some(items) = self.items.get_mut(&id) {
                            Self::process_item_updates(items, updates);
                        } else {
                            let mut items = vec![];
                            Self::process_item_updates(&mut items, updates);
                            self.items.insert(id, items);
                        }
                    }
                    RegionMessage::LogMessage(message) => {
                        println!("{}", message);
                        if self.log.is_empty() {
                            self.log = message;
                        } else {
                            self.log += &format!("{}{}", "\n", message);
                        }
                        self.log_changed = true;
                    }
                    RegionMessage::Time(id, time) => {
                        self.times.insert(id, time);
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
                let new_entity_id = new_entity.id;
                entities.push(new_entity);

                // Update the map with the new entitys ID
                entity_map.insert(new_entity_id, entities.len() - 1);
            }
        }
    }

    /// Update existing items (or create new ones if they do not exist).
    pub fn process_item_updates(items: &mut Vec<Item>, updates: Vec<ItemUpdate>) {
        // Create a mapping from entity ID to index for efficient lookup
        let mut item_map: FxHashMap<u32, usize> = items
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.id, index))
            .collect();

        for update in updates {
            if let Some(&index) = item_map.get(&update.id) {
                // Entity exists, apply the update
                items[index].apply_update(update);
            } else {
                // Entity does not exist, create a new one
                let mut new_item = Item {
                    id: update.id,
                    ..Default::default()
                };
                new_item.apply_update(update);

                // Add to the item list
                let new_entity_id = new_item.id;
                items.push(new_item);

                // Update the map with the new items ID
                item_map.insert(new_entity_id, items.len() - 1);
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
        self.clear();
    }

    /// Shuts down all region instances.
    pub fn clear(&mut self) {
        if let Ok(mut pipes) = REGIONPIPE.write() {
            pipes.clear();
        }
        if let Ok(mut players) = LOCAL_PLAYERS.write() {
            players.clear();
        }
        self.entities.clear();
        self.items.clear();
        self.id_gen = 0;
        self.region_id_map.clear();
        self.state = ServerState::Off;
        self.from_region.clear();
        self.times.clear();
        self.clear_log();
    }

    /// Create a id
    pub fn get_next_id(&mut self) -> u32 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }
}
