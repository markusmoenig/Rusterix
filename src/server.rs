pub mod assets;
pub mod entity;
pub mod message;
pub mod region;

use crate::EntityAction;
use crossbeam_channel::{Receiver, Sender};

use std::sync::{Arc, LazyLock, RwLock};
use theframework::prelude::FxHashMap;

use crate::prelude::*;

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

/// Send from a player script (either locally or remotely) to perform the given action.
fn player_action(region_id: u32, entity_id: u32, action: i32) {
    if let Some(action) = EntityAction::from_i32(action) {
        if let Ok(pipes) = REGIONPIPE.read() {
            if let Some(sender) = pipes.get(&region_id) {
                sender
                    .send(RegionMessage::UserAction(entity_id, action))
                    .unwrap();
            }
        }
    }
}

pub struct Server {
    pub id_gen: u32,
    from_region: Vec<Receiver<RegionMessage>>,
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
            from_region: vec![],
        }
    }

    /// Create the given region.
    pub fn create_region(&mut self, name: String, map: Map, entities: &FxHashMap<String, String>) {
        let mut region = Region::default();
        region.id = self.get_next_id();

        if let Ok(mut pipes) = REGIONPIPE.write() {
            pipes.insert(region.id, region.to_sender.clone());
        }

        self.from_region.push(region.from_receiver.clone());

        region.init(name, map, entities);
        region.run();
    }

    pub fn apply_entity_to_camera(&self, camera: &mut Box<dyn D3Camera>) {
        for receiver in &self.from_region {
            while let Ok(RegionMessage::Entity(entity)) = receiver.try_recv() {
                entity.apply_to_camera(camera);
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

    /// Create a id
    pub fn get_next_id(&mut self) -> u32 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }
}
