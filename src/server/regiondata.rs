use crate::server::py_fn::*;
use crate::{
    Assets, Currencies, Currency, Entity, EntityAction, Item, Map, MapMini, PixelSource,
    PlayerCamera, RegionInstance, RegionMessage, Value, ValueContainer,
};
use crossbeam_channel::{Receiver, Sender, select, tick, unbounded};
use rand::*;
use ref_thread_local::{RefThreadLocal, ref_thread_local};

use rustpython::vm::*;
use std::sync::{Arc, Mutex, OnceLock};
use theframework::prelude::{FxHashMap, FxHashSet, TheTime, Uuid};
use vek::num_traits::zero;

#[derive(Default)]
pub struct RegionData {
    pub region: RegionInstance,
    pub map: Map,
    pub mapmini: MapMini,

    pub blocking_tiles: FxHashSet<Uuid>,

    pub time: TheTime,
    pub region_id: u32,
    pub id_gen: u32,

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

    pub to_execute_entity: Vec<(u32, String, String)>,
    pub to_execute_item: Vec<(u32, String, String)>,

    pub error_count: u32,
    pub startup_errors: Vec<String>,

    pub delta_time: f32,

    pub config: toml::Table,
    pub assets: Assets,

    pub to_receiver: OnceLock<Receiver<RegionMessage>>,
    pub from_sender: OnceLock<Sender<RegionMessage>>,
}

impl RegionData {
    pub fn new() -> Self {
        Self::default()
    }
}
