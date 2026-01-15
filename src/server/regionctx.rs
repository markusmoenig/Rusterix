use crate::prelude::*;
use crate::vm::{Program, VMValue};
use crate::{CollisionWorld, MapMini};
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, OnceLock};
use theframework::prelude::*;
use toml::Table;
use uuid::Uuid;

#[derive(Default)]
pub struct RegionCtx {
    pub map: Map,
    pub mapmini: MapMini,
    pub collision_world: CollisionWorld,

    pub blocking_tiles: FxHashSet<Uuid>,

    pub debug_mode: bool,
    pub debug: DebugModule,
    pub curr_debug_loc: Option<(String, u32, u32)>,

    pub time: TheTime,
    pub region_id: u32,

    pub notifications_entities: Vec<(u32, i64, String)>,
    pub notifications_items: Vec<(u32, i64, String)>,

    pub ticks: i64,
    pub ticks_per_minute: u32,

    pub curr_entity_id: u32,
    pub curr_item_id: Option<u32>,

    pub entity_classes: FxHashMap<u32, String>,
    pub item_classes: FxHashMap<u32, String>,

    pub entity_player_classes: FxHashSet<String>,

    pub entity_class_data: FxHashMap<String, String>,
    pub item_class_data: FxHashMap<String, String>,

    pub entity_proximity_alerts: FxHashMap<u32, f32>,
    pub item_proximity_alerts: FxHashMap<u32, f32>,

    pub entity_state_data: FxHashMap<u32, ValueContainer>,
    pub item_state_data: FxHashMap<u32, ValueContainer>,

    pub to_execute_entity: Vec<(u32, String, VMValue)>,
    pub to_execute_item: Vec<(u32, String, VMValue)>,

    pub entity_programs: FxHashMap<String, Arc<Program>>,
    pub item_programs: FxHashMap<String, Arc<Program>>,

    pub error_count: u32,
    pub startup_errors: Vec<String>,

    pub delta_time: f32,
    pub config: Table,
    pub assets: Assets,

    pub to_receiver: OnceLock<Receiver<RegionMessage>>,
    pub from_sender: OnceLock<Sender<RegionMessage>>,

    pub health_attr: String,

    pub currencies: Currencies,
}

impl RegionCtx {
    /// Search for a mutable reference to an entity with the given ID.
    pub fn get_entity_mut(&mut self, entity_id: u32) -> Option<&mut Entity> {
        self.map
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
    }

    /// Search for a mutable reference to the current entity.
    pub fn get_current_entity_mut(&mut self) -> Option<&mut Entity> {
        self.map
            .entities
            .iter_mut()
            .find(|entity| entity.id == self.curr_entity_id)
    }

    /// Search for a mutable reference to an item with the given ID. Checks the map and the inventory of each entity.
    pub fn get_item_mut(&mut self, item_id: u32) -> Option<&mut Item> {
        if let Some(item) = self.map.items.iter_mut().find(|item| item.id == item_id) {
            return Some(item);
        }

        // Look in each entity’s inventory
        for entity in self.map.entities.iter_mut() {
            for item in entity.inventory.iter_mut() {
                if let Some(item) = item {
                    if item.id == item_id {
                        return Some(item);
                    }
                }
            }
        }
        None
    }

    /// Search for a mutable reference to the current item.
    pub fn get_current_item_mut(&mut self) -> Option<&mut Item> {
        self.curr_item_id.and_then(|id| self.get_item_mut(id))
    }
}
