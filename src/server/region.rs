use crate::server::py_fn::*;
use crate::{
    Assets, Currencies, Currency, Entity, EntityAction, Item, Map, MapMini, PixelSource,
    PlayerCamera, Value, ValueContainer,
};
use crossbeam_channel::{Receiver, Sender, select, tick, unbounded};
use rand::*;
use ref_thread_local::{RefThreadLocal, ref_thread_local};

use rustpython::vm::*;
use std::sync::{Arc, Mutex, OnceLock};
use theframework::prelude::{FxHashMap, FxHashSet, TheTime, Uuid};
use vek::num_traits::zero;

use std::sync::atomic::{AtomicU32, Ordering};
use vek::Vec2;

// Global Id Generator over all threads and regions
static GLOBAL_ID_GEN: AtomicU32 = AtomicU32::new(0);

pub fn get_global_id() -> u32 {
    GLOBAL_ID_GEN.fetch_add(1, Ordering::Relaxed)
}

use EntityAction::*;

// Local thread global data for the Region
ref_thread_local! {
    pub static managed REGION: RegionInstance = RegionInstance::default();
    pub static managed MAP: Map = Map::default();
    pub static managed MAPMINI: MapMini = MapMini::default();

    /// The ids of blocking tiles (and materials). We need to know these for collision detection.
    pub static managed BLOCKING_TILES: FxHashSet<Uuid> = FxHashSet::default();

    /// The server time
    pub static managed TIME: TheTime = TheTime::default();

    /// RegionID
    pub static managed REGIONID: u32 = 0;

    /// A list of notifications to send to the given entity at the specified tick.
    pub static managed NOTIFICATIONS_ENTITIES: Vec<(u32, i64, String)> = vec![];

    /// A list of notifications to send to the given items at the specified tick.
    pub static managed NOTIFICATIONS_ITEMS: Vec<(u32, i64, String)> = vec![];

    /// The current tick
    pub static managed TICKS: i64 = 0;
    /// Ticks per in-game minute
    pub static managed TICKS_PER_MINUTE: u32 = 4;

    /// The entity id which is currently handled/executed in Python
    pub static managed CURR_ENTITYID: u32 = 0;

    /// The item id which is currently handled/executed in Python
    pub static managed CURR_ITEMID: Option<u32> = None;

    /// Maps the entity id to its class name.
    pub static managed ENTITY_CLASSES: FxHashMap<u32, String> = FxHashMap::default();

    /// Maps the item id to its class name.
    pub static managed ITEM_CLASSES: FxHashMap<u32, String> = FxHashMap::default();

    /// All Entity Classes for Players (we do not instantiate them)
    pub static managed ENTITY_PLAYER_CLASSES: FxHashSet<String> = FxHashSet::default();

    /// Maps an entity class name to its data file.
    pub static managed ENTITY_CLASS_DATA: FxHashMap<String, String> = FxHashMap::default();

    /// Maps an item class name to its data file.
    pub static managed ITEM_CLASS_DATA: FxHashMap<String, String> = FxHashMap::default();

    /// Proximity alerts for entities.
    pub static managed ENTITY_PROXIMITY_ALERTS: FxHashMap<u32, f32> = FxHashMap::default();

    /// Proximity alerts for items.
    pub static managed ITEM_PROXIMITY_ALERTS: FxHashMap<u32, f32> = FxHashMap::default();

    /// ENTITY and ITEM state data, for example which events got last executed in what tick
    pub static managed ENTITY_STATE_DATA: FxHashMap<u32, ValueContainer> = FxHashMap::default();
    pub static managed ITEM_STATE_DATA: FxHashMap<u32, ValueContainer> = FxHashMap::default();

    /// Cmds which are queued to be executed to either entities or items.
    /// First String is the Cmd Id, which is used to make sure the command is executed only once a tick.
    pub static managed TO_EXECUTE_ENTITY: Vec<(u32, String, String)> = vec![];
    pub static managed TO_EXECUTE_ITEM  : Vec<(u32, String, String)> = vec![];

    /// Errors since starting the region.
    pub static managed ERROR_COUNT: u32 = 0;
    pub static managed STARTUP_ERRORS: Vec<String> = vec![];

    pub static managed DELTA_TIME: f32 = 0.0;

    /// Config TOML
    pub static managed CONFIG: toml::Table = toml::Table::default();

    /// Game Assets
    pub static managed ASSETS: Assets = Assets::default();

    pub static managed TO_RECEIVER: OnceLock<Receiver<RegionMessage>> = OnceLock::new();
    pub static managed FROM_SENDER: OnceLock<Sender<RegionMessage>> = OnceLock::new();


    // The name of the health & death arribute
    pub static managed HEALTH_ATTR: String = "HP".into();

}

use super::RegionMessage;
use super::data::{apply_entity_data, apply_item_data};
use RegionMessage::*;

pub struct RegionInstance {
    pub id: u32,

    interp: Interpreter,
    scope: Arc<Mutex<rustpython_vm::scope::Scope>>,

    name: String,

    /// Send messages to this region
    pub to_sender: Sender<RegionMessage>,
    /// Local receiver
    to_receiver: Receiver<RegionMessage>,

    /// Send messages from this region
    from_sender: Sender<RegionMessage>,
    /// Local receiver
    pub from_receiver: Receiver<RegionMessage>,
}

impl Default for RegionInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionInstance {
    pub fn new() -> Self {
        let interp = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        let scope = Arc::new(Mutex::new(interp.enter(|vm| vm.new_scope_with_builtins())));

        interp.enter(|vm| {
            let scope = scope.lock().unwrap();

            let _ = scope.globals.set_item(
                "register_player",
                vm.new_function("register_player", register_player).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_player_camera",
                vm.new_function("set_player_camera", set_player_camera)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "action",
                vm.new_function("action", player_action).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_tile",
                vm.new_function("set_tile", set_tile).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_emit_light",
                vm.new_function("set_emit_light", set_emit_light).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_rig_sequence",
                vm.new_function("set_rig_sequence", set_rig_sequence).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("take", vm.new_function("take", take).into(), vm);

            let _ = scope
                .globals
                .set_item("equip", vm.new_function("equip", equip).into(), vm);

            let _ = scope.globals.set_item(
                "get_entity_attr",
                vm.new_function("get_entity_attr", get_entity_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "get_item_attr",
                vm.new_function("get_item_attr", get_item_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "get_attr",
                vm.new_function("get_attr", get_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_attr",
                vm.new_function("set_attr", set_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "toggle_attr",
                vm.new_function("toggle_attr", toggle_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "random",
                vm.new_function("random", random_in_range).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "notify_in",
                vm.new_function("notify_in", notify_in).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "get_sector_name",
                vm.new_function("get_sector_name", get_sector_name).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "face_random",
                vm.new_function("face_random", face_random).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "random_walk",
                vm.new_function("random_walk", random_walk).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "random_walk_in_sector",
                vm.new_function("random_walk_in_sector", random_walk_in_sector)
                    .into(),
                vm,
            );

            let _ =
                scope
                    .globals
                    .set_item("message", vm.new_function("message", message).into(), vm);

            let _ = scope
                .globals
                .set_item("debug", vm.new_function("debug", debug).into(), vm);

            let _ = scope.globals.set_item(
                "inventory_items",
                vm.new_function("inventory_items", inventory_items).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "inventory_items_of",
                vm.new_function("inventory_items_of", inventory_items_of)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "entities_in_radius",
                vm.new_function("entities_in_radius", entities_in_radius)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_proximity_tracking",
                vm.new_function("set_proximity_tracking", set_proximity_tracking)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "deal_damage",
                vm.new_function("deal_damage", deal_damage).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "took_damage",
                vm.new_function("took_damage", took_damage).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "block_events",
                vm.new_function("block_events", block_events).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "add_item",
                vm.new_function("add_item", add_item).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "drop_items",
                vm.new_function("drop_items", drop_items).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "teleport",
                vm.new_function("teleport", teleport).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("goto", vm.new_function("goto", goto).into(), vm);

            let _ = scope.globals.set_item(
                "close_in",
                vm.new_function("close_in", close_in).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("id", vm.new_function("id", id).into(), vm);
        });

        let (to_sender, to_receiver) = unbounded::<RegionMessage>();
        let (from_sender, from_receiver) = unbounded::<RegionMessage>();

        Self {
            id: 0,

            interp,
            scope,

            name: String::new(),

            to_receiver,
            to_sender,
            from_receiver,
            from_sender,
        }
    }

    /// Initializes the Python bases classes, sets the map and applies entities
    pub fn init(&mut self, name: String, map: Map, assets: &Assets, config_toml: String) {
        self.name = name;

        if let Ok(toml) = config_toml.parse::<toml::Table>() {
            *CONFIG.borrow_mut() = toml;
        }

        *MAP.borrow_mut() = map;
        *NOTIFICATIONS_ENTITIES.borrow_mut() = vec![];
        *NOTIFICATIONS_ITEMS.borrow_mut() = vec![];
        *STARTUP_ERRORS.borrow_mut() = vec![];
        *BLOCKING_TILES.borrow_mut() = assets.blocking_tiles();
        *ASSETS.borrow_mut() = assets.clone();

        // Installing Entity Class Templates
        for (name, (entity_source, entity_data)) in &assets.entities {
            if let Err(err) = self.execute(entity_source) {
                STARTUP_ERRORS.borrow_mut().push(format!(
                    "{}: Error Compiling {} Character Class: {}",
                    self.name, name, err,
                ));
            }
            if let Err(err) = self.execute(&format!("{} = {}()", name, name)) {
                STARTUP_ERRORS.borrow_mut().push(format!(
                    "{}: Error Installing {} Character Class: {}",
                    self.name, name, err,
                ));
            }

            // Store entity classes which handle player
            match entity_data.parse::<toml::Table>() {
                Ok(data) => {
                    if let Some(game) = data.get("attributes").and_then(toml::Value::as_table) {
                        if let Some(value) = game.get("player") {
                            if let Some(v) = value.as_bool() {
                                if v {
                                    ENTITY_PLAYER_CLASSES.borrow_mut().insert(name.clone());
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    STARTUP_ERRORS.borrow_mut().push(format!(
                        "{}: Error Parsing {} Entity Class: {}",
                        self.name, name, err,
                    ));
                }
            }

            ENTITY_CLASS_DATA
                .borrow_mut()
                .insert(name.clone(), entity_data.clone());
        }

        // Installing Item Class Templates
        for (name, (item_source, item_data)) in &assets.items {
            if let Err(err) = self.execute(item_source) {
                STARTUP_ERRORS.borrow_mut().push(format!(
                    "{}: Error Compiling {} Item Class: {}",
                    self.name, name, err,
                ));
            }
            if let Err(err) = self.execute(&format!("{} = {}()", name, name)) {
                STARTUP_ERRORS.borrow_mut().push(format!(
                    "{}: Error Installing {} Item Class: {}",
                    self.name, name, err,
                ));
            }
            ITEM_CLASS_DATA
                .borrow_mut()
                .insert(name.clone(), item_data.clone());
        }

        // Remove player based entities, these only get created on demand from a client
        let player_classes = ENTITY_PLAYER_CLASSES.borrow().clone();
        MAP.borrow_mut()
            .entities
            .retain(|entity| match entity.get_attr_string("class_name") {
                Some(class_name) => !player_classes.contains(&class_name),
                None => true,
            });

        // Set an entity id and mark all fields dirty for the first transmission to the server.
        for e in MAP.borrow_mut().entities.iter_mut() {
            e.id = get_global_id();
            // By default we set the sequence to idle.
            e.set_attribute(
                "_source_seq",
                Value::Source(PixelSource::Sequence("idle".into())),
            );
            e.set_attribute("mode", Value::Str("active".into()));
            e.mark_all_dirty();
        }

        // Set an item id and mark all fields dirty for the first transmission to the server.
        for i in MAP.borrow_mut().items.iter_mut() {
            i.id = get_global_id();
            // By default we set the sequence to idle.
            i.attributes.set(
                "_source_seq",
                Value::Source(PixelSource::Sequence("_".into())),
            );
            i.mark_all_dirty();
        }
    }

    /// Run this instance
    pub fn run(self) {
        // We have to reassign stuff inside the thread
        let map = MAP.borrow_mut().clone();
        let name = map.name.clone();
        let startup_errors = STARTUP_ERRORS.borrow().clone();
        let entity_class_data = ENTITY_CLASS_DATA.borrow().clone();
        let entity_player_classes = ENTITY_PLAYER_CLASSES.borrow().clone();
        let item_class_data = ITEM_CLASS_DATA.borrow().clone();
        let blocking_tiles = BLOCKING_TILES.borrow().clone();
        let config = CONFIG.borrow().clone();
        let assets = ASSETS.borrow().clone();

        std::thread::spawn(move || {
            // Initialize the local thread global storage
            FROM_SENDER
                .borrow_mut()
                .set(self.from_sender.clone())
                .unwrap();
            TO_RECEIVER
                .borrow_mut()
                .set(self.to_receiver.clone())
                .unwrap();
            *REGIONID.borrow_mut() = self.id;
            *REGION.borrow_mut() = self;
            *MAPMINI.borrow_mut() = map.as_mini(&blocking_tiles);
            *MAP.borrow_mut() = map;
            *TICKS.borrow_mut() = 0;
            *TICKS_PER_MINUTE.borrow_mut() = 4;
            *ENTITY_CLASS_DATA.borrow_mut() = entity_class_data;
            *ENTITY_PLAYER_CLASSES.borrow_mut() = entity_player_classes;
            *ITEM_CLASS_DATA.borrow_mut() = item_class_data;
            *BLOCKING_TILES.borrow_mut() = blocking_tiles;
            *CONFIG.borrow_mut() = config;
            *ASSETS.borrow_mut() = assets;

            *TICKS_PER_MINUTE.borrow_mut() =
                get_config_i32_default("game", "ticks_per_minute", 4) as u32;

            let game_tick_ms = get_config_i32_default("game", "game_tick_ms", 250) as u64;
            let target_fps = get_config_i32_default("game", "target_fps", 30) as f32;

            let system_ticker = tick(std::time::Duration::from_millis(game_tick_ms));
            let redraw_ticker = tick(std::time::Duration::from_millis(
                (1000.0 / target_fps) as u64,
            ));

            *DELTA_TIME.borrow_mut() = 1.0 / target_fps;

            *HEALTH_ATTR.borrow_mut() =
                get_config_string_default("game", "health", "HP").to_string();

            let entity_block_mode = {
                let mode = get_config_string_default("game", "entity_block_mode", "always");
                if mode == "always" { 1 } else { 0 }
            };

            // Send startup messages
            *ERROR_COUNT.borrow_mut() = startup_errors.len() as u32;
            for l in startup_errors {
                send_log_message(l);
            }

            let entities = MAP.borrow().entities.clone();

            // Setting the data for the entities.
            for entity in entities.iter() {
                if let Some(class_name) = entity.get_attr_string("class_name") {
                    if let Some(data) = ENTITY_CLASS_DATA.borrow().get(&class_name) {
                        let mut map = MAP.borrow_mut();
                        for e in map.entities.iter_mut() {
                            if e.id == entity.id {
                                apply_entity_data(e, data);

                                // Fill up the inventory slots
                                if let Some(Value::Int(inv_slots)) =
                                    e.attributes.get("inventory_slots")
                                {
                                    e.inventory = vec![];
                                    for _ in 0..*inv_slots {
                                        e.inventory.push(None);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Send "startup" event to all entities.
            for entity in entities.iter() {
                if let Some(class_name) = entity.get_attr_string("class_name") {
                    let cmd = format!("{}.event(\"startup\", \"\")", class_name);
                    ENTITY_CLASSES
                        .borrow_mut()
                        .insert(entity.id, class_name.clone());
                    *CURR_ENTITYID.borrow_mut() = entity.id;
                    if let Err(err) = REGION.borrow_mut().execute(&cmd) {
                        send_log_message(format!(
                            "{}: Event Error ({}) for '{}': {}",
                            name,
                            "startup",
                            get_entity_name(entity.id),
                            err,
                        ));
                    }

                    // Determine, set and notify the entity about the sector it is in.
                    let mut sector_name = String::new();
                    if let Some(sector) = MAP.borrow().find_sector_at(entity.get_pos_xz()) {
                        sector_name = sector.name.clone();
                    }
                    {
                        let mut map = MAP.borrow_mut();
                        for e in map.entities.iter_mut() {
                            if e.id == entity.id {
                                e.attributes.set("sector", Value::Str(sector_name.clone()));
                            }
                        }
                    }
                    if !sector_name.is_empty() {
                        let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector_name);
                        _ = REGION.borrow_mut().execute(&cmd);
                    }
                }
            }

            // Send "startup" event to all items.
            let items = MAP.borrow().items.clone();
            for item in items.iter() {
                if let Some(class_name) = item.get_attr_string("class_name") {
                    let cmd = format!("{}.event(\"startup\", \"\")", class_name);
                    ITEM_CLASSES
                        .borrow_mut()
                        .insert(item.id, class_name.clone());
                    *CURR_ITEMID.borrow_mut() = Some(item.id);
                    if let Err(err) = REGION.borrow_mut().execute(&cmd) {
                        send_log_message(format!(
                            "{}: Item Event Error ({}) for '{}': {}",
                            name,
                            "startup",
                            get_entity_name(item.id),
                            err,
                        ));
                    }
                }
            }
            *CURR_ITEMID.borrow_mut() = None;

            // Running the character setup scripts for the class instances
            for entity in entities.iter() {
                if let Some(setup) = entity.get_attr_string("setup") {
                    if let Err(err) = REGION.borrow_mut().execute(&setup) {
                        send_log_message(format!(
                            "{}: Setup '{}/{}': {}",
                            name,
                            entity.get_attr_string("name").unwrap_or("Unknown".into()),
                            entity
                                .get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ));
                        *ERROR_COUNT.borrow_mut() += 1;
                    }

                    *CURR_ENTITYID.borrow_mut() = entity.id;
                    if let Err(err) = REGION.borrow_mut().execute("setup()") {
                        send_log_message(format!(
                            "{}: Setup '{}/{}': {}",
                            name,
                            entity.get_attr_string("name").unwrap_or("Unknown".into()),
                            entity
                                .get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ));
                        *ERROR_COUNT.borrow_mut() += 1;
                    }

                    /*
                    // Setting the data for the entity.
                    if let Some(class_name) = entity.get_attr_string("class_name") {
                        if let Some(data) = ENTITY_CLASS_DATA.borrow().get(&class_name) {
                            let mut map = MAP.borrow_mut();
                            for e in map.entities.iter_mut() {
                                if e.id == entity.id {
                                    apply_entity_data(e, data);

                                    if let Some(inv_slots) = e.attributes.get("inventory_slots") {
                                        println!("{} {}", class_name, inv_slots);
                                    }
                                }
                            }
                        }
                    }*/
                }
            }

            // Running the item setup scripts for the class instances
            let mut items = MAP.borrow().items.clone();
            for item in items.iter_mut() {
                if let Some(setup) = item.get_attr_string("setup") {
                    if let Err(err) = REGION.borrow_mut().execute(&setup) {
                        send_log_message(format!(
                            "{}: Item Setup '{}/{}': {}",
                            name,
                            item.get_attr_string("name").unwrap_or("Unknown".into()),
                            item.get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ));
                        *ERROR_COUNT.borrow_mut() += 1;
                    }

                    *CURR_ITEMID.borrow_mut() = Some(item.id);
                    if let Err(err) = REGION.borrow_mut().execute("setup()") {
                        send_log_message(format!(
                            "{}: Item Setup '{}/{}': {}",
                            name,
                            item.get_attr_string("name").unwrap_or("Unknown".into()),
                            item.get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ));
                        *ERROR_COUNT.borrow_mut() += 1;
                    }
                }
                // Setting the data for the item.
                if let Some(class_name) = item.get_attr_string("class_name") {
                    if let Some(data) = ITEM_CLASS_DATA.borrow().get(&class_name) {
                        let mut map = MAP.borrow_mut();
                        for i in map.items.iter_mut() {
                            if i.id == item.id {
                                apply_item_data(i, data);
                                *item = i.clone();
                            }
                        }
                    }
                    // Send active state
                    let cmd = format!(
                        "{}.event(\"active\", {})",
                        class_name,
                        if item.attributes.get_bool_default("active", false) {
                            "True"
                        } else {
                            "False"
                        }
                    );
                    _ = REGION.borrow_mut().execute(&cmd);
                }
            }
            *CURR_ITEMID.borrow_mut() = None;

            // Send startup log message
            send_log_message(format!(
                "{}: Startup with {} errors.",
                name,
                *ERROR_COUNT.borrow(),
            ));

            // Event loop
            loop {
                select! {
                    recv(system_ticker) -> _ => {
                        let ticks;
                        {
                            *TICKS.borrow_mut() += 1;
                            ticks = *TICKS.borrow();
                            let mut time = TIME.borrow_mut();
                            let mins = time.total_minutes();
                            *time = TheTime::from_ticks(ticks, *TICKS_PER_MINUTE.borrow());
                            if time.total_minutes() > mins {
                                // If the time changed send to server
                                FROM_SENDER
                                    .borrow()
                                    .get()
                                    .unwrap()
                                    .send(RegionMessage::Time(*REGIONID.borrow(), *time))
                                    .unwrap();
                            }
                        }

                        // Process notifications for entities. We have to do this carefully to avoid deadlocks
                        {
                            let to_process = {
                                let notifications = NOTIFICATIONS_ENTITIES.borrow_mut();
                                notifications.iter()
                                    .filter(|(_, tick, _)| *tick <= ticks)
                                    .cloned() // Clone only the relevant items
                                    .collect::<Vec<_>>() // Store them in a new list
                            };
                            for (id, _tick, notification) in &to_process {
                                if !is_entity_dead(*id) {
                                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(id) {
                                        let cmd = format!("{}.event(\"{}\", \"\")", class_name, notification);
                                        *CURR_ENTITYID.borrow_mut() = *id;
                                        *CURR_ITEMID.borrow_mut() = None;
                                        let _ = REGION.borrow_mut().execute(&cmd);
                                    }
                                }
                            }
                            NOTIFICATIONS_ENTITIES.borrow_mut().retain(|(id, tick, _)| !to_process.iter().any(|(pid, _, _)| pid == id && *tick <= ticks));
                        }

                        // Process notifications for items. We have to do this carefully to avoid deadlocks
                        {
                            let to_process = {
                                let notifications = NOTIFICATIONS_ITEMS.borrow_mut();
                                notifications.iter()
                                    .filter(|(_, tick, _)| *tick <= ticks)
                                    .cloned()
                                    .collect::<Vec<_>>()
                            };
                            for (id, _tick, notification) in &to_process {
                                if let Some(class_name) = ITEM_CLASSES.borrow().get(id) {
                                    let cmd = format!("{}.event(\"{}\", \"\")", class_name, notification);
                                    *CURR_ITEMID.borrow_mut() = Some(*id);
                                    let _ = REGION.borrow_mut().execute(&cmd);
                                    *CURR_ITEMID.borrow_mut() = None;
                                }
                            }
                            NOTIFICATIONS_ITEMS.borrow_mut().retain(|(id, tick, _)| !to_process.iter().any(|(pid, _, _)| pid == id && *tick <= ticks));
                        }

                        // Check Proximity Alerts
                        {
                            for (id, radius) in ENTITY_PROXIMITY_ALERTS.borrow().iter() {
                                let entities = entities_in_radius_internal(Some(*id), None, *radius);
                                if !entities.is_empty() {
                                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(id) {
                                        let cmd = format!("{}.event(\"{}\", [{}])", class_name, "proximity_warning",
                                            entities.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(","));
                                        TO_EXECUTE_ENTITY.borrow_mut().push((*id, "proximity_warning".into(), cmd));
                                    }
                                }
                            }
                        }
                    }
                    recv(redraw_ticker) -> _ => {
                        REGION.borrow_mut().handle_redraw_tick(entity_block_mode);

                        // Execute delayed scripts for entities
                        // This is because we can only borrow REGION once.
                        let to_execute_entity = TO_EXECUTE_ENTITY.borrow().clone();
                        TO_EXECUTE_ENTITY.borrow_mut().clear();
                        for todo in to_execute_entity {
                            *CURR_ENTITYID.borrow_mut() = todo.0;
                            *CURR_ITEMID.borrow_mut() = None;

                            {
                                let mut state_data = ENTITY_STATE_DATA.borrow_mut().clone();
                                if let Some(state_data) = state_data.get_mut(&todo.0) {
                                    // Check if we have already executed this script in this tick
                                    if let Some(Value::Int64(tick)) = state_data.get(&todo.1) {
                                        if *tick >= *TICKS.borrow() {
                                            if todo.1 =="intent" {
                                                send_message(todo.0, "cant_do_that_yet".into(), "warning");
                                            }
                                            continue;
                                        }
                                    }
                                    // Store the tick we executed this in
                                    state_data.set(&todo.1, Value::Int64(*TICKS.borrow()));
                                } else {
                                    let mut vc = ValueContainer::default();
                                    vc.set(&todo.1, Value::Int64(*TICKS.borrow()));
                                    state_data.insert(todo.0, vc);
                                }
                            }

                            if let Err(err) = REGION.borrow().execute(&todo.2) {
                                send_log_message(format!(
                                    "TO_EXECUTE_ENTITY: Error for '{}': {}: {}",
                                    todo.0,
                                    todo.1,
                                    err,
                                ));
                            }
                        }

                        // Execute delayed scrips for items.
                        // This is because we can only borrow REGION once.
                        let to_execute_items = TO_EXECUTE_ITEM.borrow().clone();
                        TO_EXECUTE_ITEM.borrow_mut().clear();
                        for todo in to_execute_items {
                            *CURR_ITEMID.borrow_mut() = Some(todo.0);
                            {
                                let mut state_data = ITEM_STATE_DATA.borrow_mut();
                                if let Some(state_data) = state_data.get_mut(&todo.0) {
                                    // Check if we have already executed this script in this tick
                                    if let Some(Value::Int64(tick)) = state_data.get(&todo.1) {
                                        if *tick >= *TICKS.borrow() {
                                            continue;
                                        }
                                    }
                                    // Store the tick we executed this in
                                    state_data.set(&todo.1, Value::Int64(*TICKS.borrow()));
                                } else {
                                    let mut vc = ValueContainer::default();
                                    vc.set(&todo.1, Value::Int64(*TICKS.borrow()));
                                    state_data.insert(todo.0, vc);
                                }
                            }
                            if let Err(err) = REGION.borrow().execute(&todo.2) {
                                send_log_message(format!(
                                    "TO_EXECUTE_ITEM: Error for '{}': {}: {}",
                                    todo.0,
                                    todo.1,
                                    err,
                                ));
                            }
                        }
                    },
                    recv(TO_RECEIVER.borrow().get().unwrap()) -> mess => {
                        if let Ok(message) = mess {
                            match message {
                                Event(entity_id, event, value) => {
                                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity_id) {
                                        let cmd = format!("{}.event('{}', {})", class_name, event, value);
                                        *CURR_ENTITYID.borrow_mut() = entity_id;
                                        *CURR_ITEMID.borrow_mut() = None;
                                        if let Err(err) = REGION.borrow().execute(&cmd) {
                                            send_log_message(format!(
                                                "{}: Event Error for '{}': {}",
                                                name,
                                                get_entity_name(entity_id),
                                                err,
                                            ));
                                        }
                                    }
                                }
                                UserEvent(entity_id, event, value) => {
                                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity_id) {
                                        let cmd = format!("{}.user_event('{}', '{}')", class_name, event, value);
                                        *CURR_ENTITYID.borrow_mut() = entity_id;
                                        *CURR_ITEMID.borrow_mut() = None;
                                        if let Err(err) = REGION.borrow().execute(&cmd) {
                                            send_log_message(format!(
                                                "{}: User Event Error for '{}': {}",
                                                name,
                                                get_entity_name(entity_id),
                                                err,
                                            ));
                                        }
                                    }
                                }
                                UserAction(entity_id, action) => {
                                    match action {
                                        Intent(intent) => {
                                            if let Some(entity) = MAP
                                                .borrow_mut()
                                                .entities
                                                .iter_mut()
                                                .find(|entity| entity.id == entity_id)
                                            {
                                                entity.set_attribute("intent", Value::Str(intent));
                                            }
                                        },
                                        EntityClicked(clicked_entity_id, distance) => {
                                            if let Some(entity) = MAP
                                                .borrow_mut()
                                                .entities
                                                .iter_mut()
                                                .find(|entity| entity.id == entity_id)
                                                {
                                                    send_entity_intent_events_clicked(entity, clicked_entity_id, distance);
                                                }
                                        }
                                        ItemClicked(clicked_item_id, distance) => {
                                            if let Some(entity) = MAP
                                                .borrow_mut()
                                                .entities
                                                .iter_mut()
                                                .find(|entity| entity.id == entity_id)
                                                {
                                                    send_item_intent_events_clicked(entity, clicked_item_id, distance);
                                                }
                                        }
                                        _ => {
                                            if let Some(entity) = MAP
                                                .borrow_mut()
                                                .entities
                                                .iter_mut()
                                                .find(|entity| entity.id == entity_id)
                                            {
                                                entity.action = action;
                                            }
                                        }
                                    }
                                }
                                CreateEntity(_id, entity) => {
                                    create_entity_instance(entity)
                                }
                                TransferEntity(_region_id, entity, _dest_region_name, dest_sector_name) => {
                                    receive_entity(entity, dest_sector_name);
                                }
                                Time(_id, time) => {
                                    // User manually set the server time
                                    *TICKS.borrow_mut() = time.to_ticks( *TICKS_PER_MINUTE.borrow());
                                    *TIME.borrow_mut() = time;
                                }
                                Quit => {
                                    println!("Shutting down '{}'. Goodbye.", MAP.borrow().name);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        });
    }

    /// Redraw tick
    fn handle_redraw_tick(&mut self, entity_block_mode: i32) {
        let mut updates: Vec<Vec<u8>> = vec![];
        let mut item_updates: Vec<Vec<u8>> = vec![];
        let mut entities = MAP.borrow().entities.clone();

        for entity in &mut entities {
            match &entity.action {
                EntityAction::Forward => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_north();
                                }
                                self.move_entity(entity, 1.0, entity_block_mode);
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_north();
                            let position = entity.get_forward_pos(1.0);
                            send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, 1.0, entity_block_mode);
                    }
                }
                EntityAction::Left => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_west();
                                    self.move_entity(entity, 1.0, entity_block_mode);
                                } else {
                                    entity.turn_left(4.0);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_west();
                            let position = entity.get_forward_pos(1.0);
                            send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.turn_left(4.0);
                    }
                }
                EntityAction::Right => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            // If no intent we walk
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_east();
                                    self.move_entity(entity, 1.0, entity_block_mode);
                                } else {
                                    entity.turn_right(4.0);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_east();
                            let position = entity.get_forward_pos(1.0);
                            send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.turn_right(4.0);
                    }
                }
                EntityAction::Backward => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_south();
                                    self.move_entity(entity, 1.0, entity_block_mode);
                                } else {
                                    self.move_entity(entity, -1.0, entity_block_mode);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_south();
                            let position = entity.get_forward_pos(1.0);
                            send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, -1.0, entity_block_mode);
                    }
                }
                EntityAction::CloseIn(target, target_radius, speed) => {
                    if is_entity_dead(*target) {
                        continue;
                    }

                    let speed = 4.0 * speed * *DELTA_TIME.borrow();
                    let position = entity.get_pos_xz();
                    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                    let target_id = *target;

                    let mut coord: Option<vek::Vec2<f32>> = None;

                    {
                        if let Some(entity) = MAP
                            .borrow()
                            .entities
                            .iter()
                            .find(|entity| entity.id == *target)
                        {
                            coord = Some(entity.get_pos_xz());
                        }
                    }

                    if let Some(coord) = coord {
                        let (new_position, arrived) = MAPMINI.borrow().close_in(
                            position,
                            coord,
                            *target_radius,
                            speed,
                            radius,
                            1.0,
                        );

                        entity.set_pos_xz(new_position);
                        if arrived {
                            entity.action = EntityAction::Off;

                            // Send closed in event
                            if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                                let cmd =
                                    format!("{}.event(\"closed_in\", {})", class_name, target_id);
                                TO_EXECUTE_ENTITY.borrow_mut().push((
                                    entity.id,
                                    "closed_in".into(),
                                    cmd,
                                ));
                            }
                        }

                        let map: ref_thread_local::Ref<'_, Map> = MAP.borrow();
                        check_player_for_section_change(&map, entity);
                    }
                }
                EntityAction::Goto(coord, speed) => {
                    let speed = 4.0 * speed * *DELTA_TIME.borrow();
                    let position = entity.get_pos_xz();
                    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                    let (new_position, arrived) = MAPMINI
                        .borrow()
                        .move_towards(position, *coord, speed, radius, 1.0);

                    entity.set_pos_xz(new_position);
                    if arrived {
                        entity.action = EntityAction::Off;

                        let mut sector_name: String = String::new();
                        {
                            let map = MAP.borrow();
                            if let Some(s) = map.find_sector_at(new_position) {
                                sector_name = s.name.clone();
                            }
                        }

                        // Send arrived event
                        if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                            let cmd =
                                format!("{}.event('arrived', \"{}\")", class_name, sector_name);
                            TO_EXECUTE_ENTITY.borrow_mut().push((
                                entity.id,
                                "arrived".into(),
                                cmd.clone(),
                            ));
                        }
                    };

                    let map: ref_thread_local::Ref<'_, Map> = MAP.borrow();
                    check_player_for_section_change(&map, entity);
                }
                EntityAction::RandomWalk(distance, speed, max_sleep, state, target) => {
                    if *state == 0 {
                        // State 0: Uninitialized, find a target location.
                        let pos = find_random_position(entity.get_pos_xz(), *distance);
                        entity.action = RandomWalk(*distance, *speed, *max_sleep, 1, pos);
                        entity.face_at(pos);
                    } else if *state == 1 {
                        // State 1: Walk towards
                        if target.distance(entity.get_pos_xz()) < 0.1 {
                            // Arrived, Sleep
                            let mut rng = rand::rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.random_range(*max_sleep / 2..=*max_sleep) as u32,
                                RandomWalk(*distance, *speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let t = RandomWalk(*distance, *speed, *max_sleep, 0, *target);
                            let max_sleep = *max_sleep;
                            let blocked = self.move_entity(entity, 1.0, entity_block_mode);
                            if blocked {
                                let mut rng = rand::rng();
                                entity.action = self.create_sleep_switch_action(
                                    rng.random_range(max_sleep / 2..=max_sleep) as u32,
                                    t,
                                );
                            }
                        }
                    }
                }
                EntityAction::RandomWalkInSector(distance, speed, max_sleep, state, target) => {
                    if *state == 0 {
                        // State 0: Uninitialized, find a target location.
                        let map: ref_thread_local::Ref<'_, Map> = MAP.borrow();
                        let curr_pos = entity.get_pos_xz();
                        if let Some(sector) = map.find_sector_at(curr_pos) {
                            let mut new_pos = find_random_position(curr_pos, *distance);
                            let mut found = false;

                            for _ in 0..10 {
                                if sector.is_inside(&map, new_pos) {
                                    found = true;
                                    break;
                                } else {
                                    new_pos = find_random_position(curr_pos, *distance);
                                }
                            }

                            if found {
                                entity.action =
                                    RandomWalkInSector(*distance, *speed, *max_sleep, 1, new_pos);
                                entity.face_at(new_pos);
                            } else {
                                entity.action =
                                    RandomWalkInSector(*distance, *speed, *max_sleep, 0, curr_pos);
                            }
                        }
                    } else if *state == 1 {
                        // State 1: Walk towards
                        if target.distance(entity.get_pos_xz()) < 0.1 {
                            // Arrived, Sleep
                            let mut rng = rand::rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.random_range(*max_sleep / 2..=*max_sleep) as u32,
                                RandomWalkInSector(*distance, *speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let t = RandomWalkInSector(*distance, *speed, *max_sleep, 0, *target);
                            let max_sleep = *max_sleep;
                            let blocked = self.move_entity(entity, 1.0, entity_block_mode);
                            if blocked {
                                let mut rng = rand::rng();
                                entity.action = self.create_sleep_switch_action(
                                    rng.random_range(max_sleep / 2..=max_sleep) as u32,
                                    t,
                                );
                            }
                        }
                    }
                }
                SleepAndSwitch(tick, action) => {
                    if *tick <= *TICKS.borrow() {
                        entity.action = *action.clone();
                    }
                }
                _ => {}
            }
            if entity.is_dirty() {
                updates.push(entity.get_update().pack());
                entity.clear_dirty();
            }
        }
        MAP.borrow_mut().entities = entities;

        // Send the entity updates if non empty
        if !updates.is_empty() {
            FROM_SENDER
                .borrow()
                .get()
                .unwrap()
                .send(RegionMessage::EntitiesUpdate(self.id, updates))
                .unwrap();
        }

        // let mut items = MAP.borrow().items.clone();
        for item in &mut MAP.borrow_mut().items {
            if item.is_dirty() {
                item_updates.push(item.get_update().pack());
                item.clear_dirty();
            }
        }

        // Send the item updates if non empty
        if !item_updates.is_empty() {
            FROM_SENDER
                .borrow()
                .get()
                .unwrap()
                .send(RegionMessage::ItemsUpdate(self.id, item_updates))
                .unwrap();
        }
    }

    /// Execute a script.
    pub fn execute(&self, source: &str) -> Result<PyObjectRef, String> {
        let scope = self.scope.lock().unwrap();

        self.interp.enter(|vm| {
            let rc = vm.run_block_expr(scope.clone(), source);
            match rc {
                Ok(obj) => Ok(obj),
                Err(error) => {
                    let mut err_line: Option<u32> = None;

                    if let Some(tb) = error.__traceback__() {
                        // let file_name = tb.frame.code.source_path.as_str();
                        let instruction_index =
                            tb.frame.lasti.load(std::sync::atomic::Ordering::Relaxed);
                        err_line = Some(instruction_index / 2);
                        // let function_name = tb.frame.code.obj_name.as_str();
                    }

                    let mut err_string = String::new();
                    if let Some(err) = error.args().first() {
                        if let Ok(msg) = err.str(vm) {
                            err_string = msg.to_string();
                        }
                    }

                    if let Some(err_line) = err_line {
                        err_string = format!("{} at line {}.", err_string, err_line);
                    }
                    Err(err_string)
                }
            }
        })
    }

    /// Create a sleep action which switches back to the previous action.
    fn create_sleep_switch_action(&self, minutes: u32, switchback: EntityAction) -> EntityAction {
        let tick = *TICKS.borrow() + (minutes * *TICKS_PER_MINUTE.borrow()) as i64;
        SleepAndSwitch(tick, Box::new(switchback))
    }

    // Moves an entity forward or backward. Returns true if blocked.
    /*
    fn move_entity(&self, entity: &mut Entity, dir: f32, entity_block_mode: i32) -> bool {
        let speed = 4.0 * *DELTA_TIME.borrow();
        let move_vector = entity.orientation * speed * dir;
        let position = entity.get_pos_xz();
        let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;

        // Collision detection with other entities and items
        {
            let map = &MAP.borrow();
            let new_position = position + move_vector;

            for other in map.entities.iter() {
                if other.id == entity.id || !other.attributes.get_bool_default("visible", false) {
                    continue;
                }

                let other_position = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;

                let distance_squared = (new_position - other_position).magnitude_squared();
                let combined_radius = radius + other_radius;
                let combined_radius_squared = combined_radius * combined_radius;

                // Collision with another entity ?
                if distance_squared < combined_radius_squared {
                    // Send "bumped_into_entity" for the moving entity
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_into_entity", other.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            entity.id,
                            "bumped_into_entity".into(),
                            cmd,
                        ));
                    }
                    // Send "bumped_by_entity" for the other entity
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&other.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_by_entity", entity.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            other.id,
                            "bumped_by_entity".into(),
                            cmd,
                        ));
                    }
                    // if the other entity is blocking, stop the movement
                    // if other.attributes.get_bool_default("blocking", false) {
                    //     return true;
                    // }
                    if entity_block_mode > 0 {
                        return true;
                    }
                }
            }

            // Collision with an item ?
            for other in map.items.iter() {
                // If not visible, skip
                if !other.attributes.get_bool_default("visible", false) {
                    continue;
                }

                let other_position = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;

                let distance_squared = (new_position - other_position).magnitude_squared();
                let combined_radius = radius + other_radius;
                let combined_radius_squared = combined_radius * combined_radius;

                if distance_squared < combined_radius_squared {
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_into_item", other.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            entity.id,
                            "bumped_into_item".into(),
                            cmd,
                        ));
                    }
                    if let Some(class_name) = ITEM_CLASSES.borrow().get(&other.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_by_entity", entity.id
                        );
                        TO_EXECUTE_ITEM.borrow_mut().push((
                            other.id,
                            "bumped_by_entity".into(),
                            cmd,
                        ));
                    }
                    // If the item is blocking, stop the movement
                    if other.attributes.get_bool_default("blocking", false) {
                        return true;
                    }
                }
            }
            entity.set_pos_xz(new_position);
        }

        // Test against geometry (linedefs)

        let (end_position, blocked) = MAPMINI
            .borrow()
            .move_distance(position, move_vector, radius);
        entity.set_pos_xz(end_position);

        blocked
    }*/

    /// Moves an entity forward or backward. Returns true if blocked.
    fn move_entity(&self, entity: &mut Entity, dir: f32, entity_block_mode: i32) -> bool {
        let speed = 4.0 * *DELTA_TIME.borrow();
        let move_vector = entity.orientation * speed * dir;
        let position = entity.get_pos_xz();
        let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;

        let mut new_position = position + move_vector;
        let map = &MAP.borrow();

        // We'll do up to N attempts to resolve collisions via sliding
        const MAX_ITERATIONS: usize = 5;

        for _attempt in 0..MAX_ITERATIONS {
            let mut pushed = false; // Track if we had to push/slide this iteration

            // 1) Check collisions with ENTITIES
            for other in map.entities.iter() {
                if other.id == entity.id || other.get_mode() == "dead" {
                    continue;
                }

                let other_pos = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
                let combined_radius = radius + other_radius;
                let combined_radius_sq = combined_radius * combined_radius;

                // Are we colliding now?
                let dist_vec = new_position - other_pos;
                let dist_sq = dist_vec.magnitude_squared();
                if dist_sq < combined_radius_sq {
                    // Send events
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_into_entity", other.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            entity.id,
                            "bumped_into_entity".into(),
                            cmd,
                        ));
                    }
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&other.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_by_entity", entity.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            other.id,
                            "bumped_by_entity".into(),
                            cmd,
                        ));
                    }

                    // If blocking, we attempt to slide
                    if entity_block_mode > 0 {
                        // Normal from the obstacle center to the entity
                        let normal = dist_vec.normalized();

                        let total_move = new_position - position;
                        let slide = total_move - normal * total_move.dot(normal);

                        let slide_pos = position + slide;
                        let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                        if slide_dist_sq >= combined_radius_sq {
                            // We successfully slid away
                            new_position = slide_pos;
                        } else {
                            // If even after sliding we still collide, we push out just enough
                            // to stand exactly at the boundary
                            let actual_dist = (slide_pos - other_pos).magnitude();
                            if actual_dist < combined_radius {
                                let push_amount = combined_radius - actual_dist;
                                new_position = slide_pos + normal * push_amount;
                                // Re-check again next iteration
                            }
                        }
                        pushed = true;
                    }
                }
            }

            // 2) Check collisions with ITEMS
            for other in map.items.iter() {
                if !other.attributes.get_bool_default("visible", false) {
                    continue;
                }

                let other_pos = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
                let combined_radius = radius + other_radius;
                let combined_radius_sq = combined_radius * combined_radius;

                let dist_vec = new_position - other_pos;
                let dist_sq = dist_vec.magnitude_squared();
                if dist_sq < combined_radius_sq {
                    // Send events
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_into_item", other.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            entity.id,
                            "bumped_into_item".into(),
                            cmd,
                        ));
                    }
                    if let Some(class_name) = ITEM_CLASSES.borrow().get(&other.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_by_entity", entity.id
                        );
                        TO_EXECUTE_ITEM.borrow_mut().push((
                            other.id,
                            "bumped_by_entity".into(),
                            cmd,
                        ));
                    }

                    // If item is blocking, we attempt to slide
                    if other.attributes.get_bool_default("blocking", false) {
                        let normal = dist_vec.normalized();

                        let total_move = new_position - position;
                        let slide = total_move - normal * total_move.dot(normal);

                        let slide_pos = position + slide;
                        let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                        if slide_dist_sq >= combined_radius_sq {
                            // we successfully slid away
                            new_position = slide_pos;
                        } else {
                            // If still colliding, push to boundary
                            let actual_dist = (slide_pos - other_pos).magnitude();
                            if actual_dist < combined_radius {
                                let push_amount = combined_radius - actual_dist;
                                new_position = slide_pos + normal * push_amount;
                                // We'll re-check next iteration
                            }
                        }
                        pushed = true;
                    }
                }
            }

            // If we didn't have to push at all, we’re clear => break early
            if !pushed {
                break;
            }
        }

        // Now we set the new position after we've done all the entity/item collision resolution
        entity.set_pos_xz(new_position);

        entity.position.y = map
            .terrain
            .sample_height_bilinear(entity.position.x, entity.position.z)
            + 1.5;

        // Finally, let the geometry/linedef collision do its thing
        let (end_position, geometry_blocked) =
            MAPMINI
                .borrow()
                .move_distance(position, new_position - position, radius);

        // Move the entity after geometry
        entity.set_pos_xz(end_position);

        check_player_for_section_change(map, entity);

        geometry_blocked
    }
}

/// Send "intent" events for the entity or item at the given position.
fn send_entity_intent_events(entity: &mut Entity, position: Vec2<f32>) {
    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
        // Send "intent" event for the entity
        let mut cont = ValueContainer::default();
        cont.set("distance", Value::Float(1.0));

        let mut item_id = None;

        let mut found_target = false;
        if let Some(entity_id) = get_entity_at(position) {
            if entity_id != entity.id {
                cont.set("entity_id", Value::UInt(entity_id));
                found_target = true;
            }
        }
        if let Some(i_id) = get_item_at(position) {
            cont.set("entity_id", Value::UInt(entity.id));
            cont.set("item_id", Value::UInt(i_id));
            item_id = Some(i_id);
            found_target = true;
        }

        let intent = entity.attributes.get_str_default("intent", "".into());

        if !found_target {
            let message = format!("nothing_to_{}", intent);
            entity.set_attribute("intent", Value::Str(String::new()));
            send_message(entity.id, message, "system");
            return;
        }

        cont.set("intent", Value::Str(intent));
        let cmd = format!(
            "{}.event('intent', {})",
            class_name,
            cont.to_python_dict_string()
        );
        TO_EXECUTE_ENTITY
            .borrow_mut()
            .push((entity.id, "intent".into(), cmd.clone()));

        if let Some(item_id) = item_id {
            if let Some(class_name) = ITEM_CLASSES.borrow().get(&item_id) {
                let cmd = format!(
                    "{}.event('intent', {})",
                    class_name,
                    cont.to_python_dict_string()
                );
                TO_EXECUTE_ITEM
                    .borrow_mut()
                    .push((item_id, "intent".into(), cmd));
            }
        }

        entity.set_attribute("intent", Value::Str(String::new()));
    }
}

/// Player clicked on an entity.
fn send_entity_intent_events_clicked(entity: &mut Entity, target: u32, distance: f32) {
    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
        // Send "intent" event for the entity
        let mut cont = ValueContainer::default();
        cont.set("distance", Value::Float(distance));
        cont.set("entity_id", Value::UInt(target));

        let intent = entity.attributes.get_str_default("intent", "".into());

        cont.set("intent", Value::Str(intent));
        let cmd = format!(
            "{}.event('intent', {})",
            class_name,
            cont.to_python_dict_string()
        );
        TO_EXECUTE_ENTITY
            .borrow_mut()
            .push((entity.id, "intent".into(), cmd.clone()));

        entity.set_attribute("intent", Value::Str(String::new()));
    }
}

/// Player clicked on an item.
fn send_item_intent_events_clicked(entity: &mut Entity, target: u32, distance: f32) {
    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
        // Send "intent" event for the entity
        let mut cont = ValueContainer::default();
        cont.set("distance", Value::Float(distance));
        cont.set("item_id", Value::UInt(target));
        cont.set("entity_id", Value::UInt(entity.id));

        let intent = entity.attributes.get_str_default("intent", "".into());

        cont.set("intent", Value::Str(intent));
        let cmd = format!(
            "{}.event('intent', {})",
            class_name,
            cont.to_python_dict_string()
        );
        TO_EXECUTE_ENTITY
            .borrow_mut()
            .push((entity.id, "intent".into(), cmd.clone()));

        if let Some(class_name) = ITEM_CLASSES.borrow().get(&target) {
            let cmd = format!(
                "{}.event('intent', {})",
                class_name,
                cont.to_python_dict_string()
            );
            TO_EXECUTE_ITEM
                .borrow_mut()
                .push((target, "intent".into(), cmd));
        }

        entity.set_attribute("intent", Value::Str(String::new()));
    }
}

/// Check if the player moved to a different sector and if yes send "enter" and "left" events
fn check_player_for_section_change(map: &Map, entity: &mut Entity) {
    // Determine, set and notify the entity about the sector it is in.
    if let Some(sector) = map.find_sector_at(entity.get_pos_xz()) {
        if let Some(Value::Str(old_sector_name)) = entity.attributes.get("sector") {
            if sector.name != *old_sector_name {
                if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                    // Send entered event
                    if !sector.name.is_empty() {
                        let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector.name);
                        // println!("{cmd}");
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            entity.id,
                            "bumped_into_item".into(),
                            cmd,
                        ));
                    }
                    // Send left event
                    if !old_sector_name.is_empty() {
                        let cmd =
                            format!("{}.event(\"left\", \"{}\")", class_name, old_sector_name);
                        // println!("{cmd}");
                        TO_EXECUTE_ENTITY.borrow_mut().push((
                            entity.id,
                            "bumped_into_item".into(),
                            cmd,
                        ));
                    }
                }

                entity
                    .attributes
                    .set("sector", Value::Str(sector.name.clone()));
            }
        }
    } else if let Some(Value::Str(old_sector_name)) = entity.attributes.get("sector") {
        // Send left event
        if !old_sector_name.is_empty() {
            if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                let cmd = format!("{}.event(\"left\", \"{}\")", class_name, old_sector_name);
                // println!("{cmd}");
                TO_EXECUTE_ENTITY
                    .borrow_mut()
                    .push((entity.id, "bumped_into_item".into(), cmd));
            }
        }
        entity.attributes.set("sector", Value::Str(String::new()));
    }
}

/// Register Player
fn register_player() {
    let region_id = *REGIONID.borrow();
    let entity_id = *CURR_ENTITYID.borrow();

    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.set_attribute("player_camera", Value::PlayerCamera(PlayerCamera::D2));
    }

    FROM_SENDER
        .borrow()
        .get()
        .unwrap()
        .send(RegisterPlayer(region_id, entity_id))
        .unwrap();
}

/// Set Player Camera
fn set_player_camera(camera: String) {
    let player_camera = match camera.as_str() {
        "iso" => PlayerCamera::D3Iso,
        "firstp" => PlayerCamera::D3FirstP,
        _ => PlayerCamera::D2,
    };

    let entity_id = *CURR_ENTITYID.borrow();

    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.set_attribute("player_camera", Value::PlayerCamera(player_camera));
    }
}

/// Get the name of the entity with the given id.
pub fn get_entity_name(id: u32) -> String {
    for entity in &MAP.borrow().entities {
        if entity.id == id {
            if let Some(name) = entity.attributes.get_str("name") {
                return name.to_string();
            }
        }
    }
    "Unknown".into()
}

/// Is the given entity a player.
pub fn is_entity_player(id: u32) -> bool {
    for entity in &MAP.borrow().entities {
        if entity.id == id {
            return entity.is_player();
        }
    }
    false
}

/// Is the given entity dead.
pub fn is_entity_dead(id: u32) -> bool {
    for entity in &MAP.borrow().entities {
        if entity.id == id {
            return entity.attributes.get_str_default("mode", "active".into()) == "dead";
        }
    }
    false
}

/// Send a log message.
pub fn send_log_message(message: String) {
    FROM_SENDER
        .borrow()
        .get()
        .unwrap()
        .send(RegionMessage::LogMessage(message))
        .unwrap();
}

/// Perform the given action on the next update().
fn player_action(action: String) {
    if let Ok(parsed_action) = action.parse::<EntityAction>() {
        let entity_id = *CURR_ENTITYID.borrow();
        if let Some(entity) = MAP
            .borrow_mut()
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        {
            entity.action = parsed_action;
        }
    }
}

/// Sets light emission to on / off
fn set_emit_light(value: bool) {
    if let Some(item_id) = *CURR_ITEMID.borrow() {
        if let Some(item) = MAP
            .borrow_mut()
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
        {
            if let Some(Value::Light(light)) = item.attributes.get_mut("light") {
                light.active = value;
                item.mark_dirty_attribute("light");
            }
        }
    } else {
        let entity_id = *CURR_ENTITYID.borrow();
        if let Some(entity) = MAP
            .borrow_mut()
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        {
            if let Some(Value::Light(light)) = entity.attributes.get_mut("light") {
                light.active = value;
                entity.mark_dirty_attribute("light");
            }
        }
    }
}

/// Set the tile_id of the current entity or item.
fn set_tile(id: String) {
    if let Ok(uuid) = Uuid::try_parse(&id) {
        if let Some(item_id) = *CURR_ITEMID.borrow() {
            if let Some(item) = MAP
                .borrow_mut()
                .items
                .iter_mut()
                .find(|item| item.id == item_id)
            {
                item.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
            }
        } else {
            let entity_id = *CURR_ENTITYID.borrow();
            if let Some(entity) = MAP
                .borrow_mut()
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
            {
                entity.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
            }
        }
    }
}

/// Set rigging sequence
pub fn set_rig_sequence(
    args: rustpython_vm::function::FuncArgs,
    vm: &VirtualMachine,
) -> PyResult<()> {
    let mut sequence = vec![];

    for arg in args.args.iter() {
        if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
            sequence.push(v);
        }
    }

    Ok(())
}

/// Take the given item.
fn take(item_id: u32) -> bool {
    let entity_id = *CURR_ENTITYID.borrow();
    let mut map = MAP.borrow_mut();
    let mut rc = true;

    if let Some(pos) = map
        .items
        .iter()
        .position(|item| item.id == item_id && !item.attributes.get_bool_default("static", false))
    {
        let item = map.items.remove(pos);

        if let Some(entity) = map
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        {
            let mut item_name = "Unknown".to_string();
            if let Some(name) = item.attributes.get_str("name") {
                item_name = name.to_string();
            }

            fn article_for(item_name: &str) -> (&'static str, String) {
                let name = item_name.to_ascii_lowercase();

                let pair_items = ["trousers", "pants", "gloves", "boots", "scissors"];
                let mass_items = ["armor", "cloth", "water", "meat"];

                if pair_items.contains(&name.as_str()) {
                    ("a pair of", item_name.to_string())
                } else if mass_items.contains(&name.as_str()) {
                    ("some", item_name.to_string())
                } else {
                    let first = name.chars().next().unwrap_or('x');
                    let article = match first {
                        'a' | 'e' | 'i' | 'o' | 'u' => "an",
                        _ => "a",
                    };
                    (article, item_name.to_string())
                }
            }

            let mut message = format!(
                "You take {} {}",
                article_for(&item_name.to_lowercase()).0,
                item_name.to_lowercase()
            );

            if item.attributes.get_bool_default("monetary", false) {
                // This is not a standalone item but money
                let amount = item.attributes.get_float_default("worth", 0.0);
                if amount > 0.0 {
                    message = format!("You take {} gold.", amount);
                    let mut currencies = Currencies::default();
                    _ = currencies.add_currency(Currency {
                        name: "Gold".into(),
                        symbol: "G".into(),
                        exchange_rate: 1.0,
                        max_limit: None,
                    });
                    currencies.base_currency = "G".to_string();
                    _ = entity.add_base_currency(amount as i64, &currencies);
                }
            } else if entity.add_item(item).is_err() {
                // TODO: Send message.
                println!("Take: Too many items");
                rc = false;
            }
            FROM_SENDER
                .borrow()
                .get()
                .unwrap()
                .send(RegionMessage::RemoveItem(*REGIONID.borrow(), item_id))
                .unwrap();

            let msg = RegionMessage::Message(
                *REGIONID.borrow(),
                Some(entity_id),
                None,
                entity_id,
                message,
                "system".into(),
            );
            FROM_SENDER.borrow().get().unwrap().send(msg).unwrap();
        }
    }
    rc
}

/// Block the events for the entity / item for the given amount of minutes.
pub fn block_events(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) {
    let mut minutes: f32 = 4.0;
    let mut events: Vec<String> = Vec::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(v) = Value::from_pyobject(arg.clone(), vm).and_then(|v| v.to_f32()) {
                minutes = v;
            }
        } else if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
            events.push(v);
        }
    }

    let target_tick =
        Value::Int64(*TICKS.borrow() + (*TICKS_PER_MINUTE.borrow() as f32 * minutes) as i64);

    if let Some(item_id) = *CURR_ITEMID.borrow() {
        let mut state_data = ITEM_STATE_DATA.borrow_mut();
        if let Some(state_data) = state_data.get_mut(&item_id) {
            for event in events {
                state_data.set(&event, target_tick.clone());
            }
        } else {
            let mut vc = ValueContainer::default();
            for event in events {
                vc.set(&event, target_tick.clone());
            }
            state_data.insert(item_id, vc);
        }
    } else {
        let entity_id = *CURR_ENTITYID.borrow();

        let mut state_data = ENTITY_STATE_DATA.borrow_mut();
        if let Some(state_data) = state_data.get_mut(&entity_id) {
            for event in events {
                state_data.set(&event, target_tick.clone());
            }
        } else {
            let mut vc = ValueContainer::default();
            for event in events {
                vc.set(&event, target_tick.clone());
            }
            state_data.insert(entity_id, vc);
        }
    }
}

/// Deal damage to the given entity. Sends an "take_damage" event to the other entity.
fn deal_damage(id: u32, dict: PyObjectRef, vm: &VirtualMachine) {
    let dict = extract_dictionary(dict, vm);

    if let Ok(dict) = dict {
        if let Some(entity) = MAP.borrow().entities.iter().find(|entity| entity.id == id) {
            if let Some(class_name) = entity.attributes.get_str("class_name") {
                let cmd = format!("{}.event('{}', {})", class_name, "take_damage", dict);
                TO_EXECUTE_ENTITY
                    .borrow_mut()
                    .push((id, "take_damage".into(), cmd));
            }
        } else if let Some(item) = MAP.borrow_mut().items.iter_mut().find(|item| item.id == id) {
            if let Some(class_name) = item.attributes.get_str("class_name") {
                let cmd = format!("{}.event('{}', {})", class_name, "take_damage", dict);
                TO_EXECUTE_ITEM
                    .borrow_mut()
                    .push((id, "take_damage".into(), cmd));
            }
        }
    }
}

/// Send a message to the entity.
fn send_message(id: u32, message: String, role: &str) {
    // Kill message
    let msg = RegionMessage::Message(
        *REGIONID.borrow(),
        Some(id),
        None,
        id,
        message,
        role.to_string(),
    );
    FROM_SENDER.borrow().get().unwrap().send(msg).unwrap();
}

/// An entity took damage. Send out messages and check for death.
fn took_damage(from: u32, mut amount: i32) {
    let mut kill = false;
    let id = *CURR_ENTITYID.borrow();

    // Make sure we don't heal by accident
    amount = amount.max(0);
    if amount == 0 {
        return;
    }

    // Check for death
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == id)
    {
        let health_attr = HEALTH_ATTR.borrow();
        if let Some(mut health) = entity.attributes.get_int(&health_attr) {
            // Reduce the health of the target
            health -= amount;
            health = health.max(0);
            // Set the new health
            entity.set_attribute(&health_attr, Value::Int(health));

            let mode = entity.attributes.get_str_default("mode", "".into());
            if health <= 0 && mode != "dead" {
                // Send "death" event
                if let Some(class_name) = entity.attributes.get_str("class_name") {
                    let cmd = format!("{}.event(\"death\", \"\")", class_name);
                    TO_EXECUTE_ENTITY
                        .borrow_mut()
                        .push((entity.id, "death".into(), cmd));

                    entity.set_attribute("mode", Value::Str("dead".into()));
                    entity.action = EntityAction::Off;
                    ENTITY_PROXIMITY_ALERTS.borrow_mut().remove(&entity.id);

                    kill = true;
                }
            }
        }
    }

    // if receiver got killed, send a "kill" event to the attacker
    if kill {
        if let Some(entity) = MAP
            .borrow_mut()
            .entities
            .iter_mut()
            .find(|entity| from == entity.id)
        {
            // Send "kill" event
            if let Some(class_name) = entity.attributes.get_str("class_name") {
                let cmd = format!("{}.event(\"kill\", {})", class_name, id);
                TO_EXECUTE_ENTITY
                    .borrow_mut()
                    .push((from, "kill".into(), cmd));
            }
        }
    }
}

/// Get an attribute from the given entity.
fn get_entity_attr(entity_id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        if let Some(v) = entity.attributes.get(&key) {
            value = v.clone();
        }
    }

    Ok(value.to_pyobject(vm))
}

/// Get an attribute from the given entity for internal use.
fn _get_entity_attr_internal(entity_id: u32, key: String) -> Option<Value> {
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)
    {
        entity.attributes.get(&key).cloned()
    } else {
        None
    }
}

/// Get an attribute from the given item.
fn get_item_attr(item_id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    if let Some(item) = MAP
        .borrow_mut()
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
    {
        if let Some(v) = item.get_attribute(&key) {
            value = v.clone();
        }
    }

    Ok(value.to_pyobject(vm))
}

/// Get an attribute from the current item or entity.
fn get_attr(key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    if let Some(item_id) = *CURR_ITEMID.borrow() {
        if let Some(item) = MAP
            .borrow_mut()
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
        {
            if let Some(v) = item.get_attribute(&key) {
                value = v.clone();
            }
        }
    } else {
        let entity_id = *CURR_ENTITYID.borrow();
        if let Some(entity) = MAP
            .borrow_mut()
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        {
            if let Some(v) = entity.attributes.get(&key) {
                value = v.clone();
            }
        }
    }

    Ok(value.to_pyobject(vm))
}

/// Toggles a boolean attribute of the current entity or item.
fn toggle_attr(key: String) {
    if let Some(item_id) = *CURR_ITEMID.borrow() {
        if let Some(item) = MAP
            .borrow_mut()
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
        {
            item.attributes.toggle(&key);
            if key == "active" {
                // Send active state
                if let Some(class_name) = item.attributes.get_str("class_name") {
                    let cmd = format!(
                        "{}.event(\"active\", {})",
                        class_name,
                        if item.attributes.get_bool_default("active", false) {
                            "True"
                        } else {
                            "False"
                        }
                    );
                    TO_EXECUTE_ITEM
                        .borrow_mut()
                        .push((item.id, "active".into(), cmd));
                }
            }
        } else {
            let entity_id = *CURR_ENTITYID.borrow();
            if let Some(entity) = MAP
                .borrow_mut()
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
            {
                entity.attributes.toggle(&key);
            }
        }
    }
}

/// Set the attribute of the current entity or item.
fn set_attr(key: PyObjectRef, value: PyObjectRef, vm: &VirtualMachine) {
    if let Ok(key) = String::try_from_object(vm, key) {
        if let Some(value) = Value::from_pyobject(value, vm) {
            if let Some(item_id) = *CURR_ITEMID.borrow() {
                if let Some(item) = MAP
                    .borrow_mut()
                    .items
                    .iter_mut()
                    .find(|item| item.id == item_id)
                {
                    item.set_attribute(&key, value);

                    if key == "active" {
                        // Send active state
                        if let Some(class_name) = item.attributes.get_str("class_name") {
                            let cmd = format!(
                                "{}.event(\"active\", {})",
                                class_name,
                                if item.attributes.get_bool_default("active", false) {
                                    "True"
                                } else {
                                    "False"
                                }
                            );
                            TO_EXECUTE_ITEM
                                .borrow_mut()
                                .push((item.id, "active".into(), cmd));
                        }
                    }
                }
            } else {
                let entity_id = *CURR_ENTITYID.borrow();
                if let Some(entity) = MAP
                    .borrow_mut()
                    .entities
                    .iter_mut()
                    .find(|entity| entity.id == entity_id)
                {
                    entity.set_attribute(&key, value);
                }
            }
        }
    }
}

/// Returns a list of filtered inventory items.
fn inventory_items_of(
    entity_id: u32,
    filter: String,
    vm: &VirtualMachine,
) -> PyResult<PyObjectRef> {
    let mut items = Vec::new();

    let map = MAP.borrow();
    if let Some(entity) = map.entities.iter().find(|entity| entity.id == entity_id) {
        for (_, item) in entity.iter_inventory() {
            let name = item.attributes.get_str("name").unwrap_or_default();
            let class_name = item.attributes.get_str("class_name").unwrap_or_default();

            if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                items.push(item.id);
            }
        }
    }

    let py_list = vm.ctx.new_list(
        items
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Returns a list of filtered inventory items.
fn inventory_items(filter: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut items = Vec::new();

    let map = MAP.borrow();
    let entity_id = *CURR_ENTITYID.borrow();

    if let Some(entity) = map.entities.iter().find(|entity| entity.id == entity_id) {
        for (_, item) in entity.iter_inventory() {
            let name = item.attributes.get_str("name").unwrap_or_default();
            let class_name = item.attributes.get_str("class_name").unwrap_or_default();

            if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                items.push(item.id);
            }
        }
    }

    let py_list = vm.ctx.new_list(
        items
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Drop the given items.
fn drop_items(filter: String) {
    let mut map = MAP.borrow_mut();

    let entity_id = *CURR_ENTITYID.borrow();
    if let Some(entity) = map
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        // Collect matching slot indices
        let matching_slots: Vec<usize> = entity
            .iter_inventory()
            .filter_map(|(slot, item)| {
                let name = item.attributes.get_str("name").unwrap_or_default();
                let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                    Some(slot)
                } else {
                    None
                }
            })
            .collect();

        // Remove matching items from slots
        let mut removed_items = Vec::new();
        for slot in matching_slots {
            if let Some(mut item) = entity.remove_item_from_slot(slot) {
                item.position = entity.position;
                item.mark_all_dirty();
                removed_items.push(item);
            }
        }

        for item in removed_items {
            map.items.push(item);
        }
    }
}

/// Returns the entities in the radius of the character or item.
fn entities_in_radius_internal(
    entity_id: Option<u32>,
    item_id: Option<u32>,
    radius: f32,
) -> Vec<u32> {
    let mut position = None;
    let mut is_entity = false;
    let mut id = 0;

    let map = MAP.borrow();

    if let Some(item_id) = item_id {
        if let Some(item) = map.items.iter().find(|item| item.id == item_id) {
            id = item_id;
            position = Some(item.get_pos_xz());
        }
    } else if let Some(entity_id) = entity_id {
        is_entity = true;
        if let Some(entity) = map.entities.iter().find(|entity| entity.id == entity_id) {
            id = entity.id;
            position = Some(entity.get_pos_xz());
        }
    }

    let mut entities = Vec::new();

    if let Some(position) = position {
        for other in map.entities.iter() {
            if is_entity && other.id == id {
                continue;
            }
            let other_position = other.get_pos_xz();
            let other_radius = other.attributes.get_float_default("radius", 0.5);

            let distance_squared = (position - other_position).magnitude_squared();
            let combined_radius = radius + other_radius;
            let combined_radius_squared = combined_radius * combined_radius;

            // Entity is inside the radius
            if distance_squared < combined_radius_squared {
                entities.push(other.id);
            }
        }
    }

    entities
}

/// Returns the entity at the given position (if any)
fn get_entity_at(position: Vec2<f32>) -> Option<u32> {
    let map = MAP.borrow();

    let mut entity = None;

    for other in map.entities.iter() {
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            entity = Some(other.id);
            break; // We only need the first item found
        }
    }

    entity
}

/// Returns the item at the given position (if any)
fn get_item_at(position: Vec2<f32>) -> Option<u32> {
    let map = MAP.borrow();

    let mut item = None;

    for other in map.items.iter() {
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            item = Some(other.id);
            break; // We only need the first item found
        }
    }

    item
}

/// Returns the entities in the radius of the character or item.
fn entities_in_radius(vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut radius = 0.5;
    let mut position = None;
    let mut is_entity = false;
    let mut id = 0;

    let map = MAP.borrow();

    if let Some(item_id) = *CURR_ITEMID.borrow() {
        if let Some(item) = map.items.iter().find(|item| item.id == item_id) {
            id = item_id;
            position = Some(item.get_pos_xz());
            radius = item.attributes.get_float_default("radius", 0.5);
        }
    } else {
        let entity_id = *CURR_ENTITYID.borrow();
        is_entity = true;
        if let Some(entity) = map.entities.iter().find(|entity| entity.id == entity_id) {
            id = entity.id;
            position = Some(entity.get_pos_xz());
            radius = entity.attributes.get_float_default("radius", 0.5);
        }
    }

    let mut entities = Vec::new();

    if let Some(position) = position {
        for other in map.entities.iter() {
            if is_entity && other.id == id {
                continue;
            }
            let other_position = other.get_pos_xz();
            let other_radius = other.attributes.get_float_default("radius", 0.5);

            let distance_squared = (position - other_position).magnitude_squared();
            let combined_radius = radius + other_radius;
            let combined_radius_squared = combined_radius * combined_radius;

            // Entity is inside the radius
            if distance_squared < combined_radius_squared {
                entities.push(other.id);
            }
        }
    }

    let py_list = vm.ctx.new_list(
        entities
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Add an item to the characters inventory
fn add_item(class_name: String) -> i32 {
    if let Some(item) = create_item(class_name.clone()) {
        let id = *CURR_ENTITYID.borrow();
        if let Some(entity) = MAP
            .borrow_mut()
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
        {
            let item_id = item.id;
            if entity.add_item(item).is_ok() {
                item_id as i32
            } else {
                println!("add_item ({}): Inventory is full", class_name);
                -1
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Equip the item with the given item id.
fn equip(item_id: u32) {
    let id = *CURR_ENTITYID.borrow();
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == id)
    {
        let mut slot: Option<String> = None;
        if let Some(item) = entity.get_item(item_id) {
            if let Some(sl) = item.attributes.get_str("slot") {
                slot = Some(sl.to_string());
            }
        }

        if let Some(slot) = slot {
            if entity.equip_item(item_id, &slot).is_err() {
                println!("Equipped failure");
            }
        }
    }
}

/// Notify the entity / item in the given amount of minutes.
fn notify_in(minutes: i32, notification: String) {
    let tick = *TICKS.borrow() + (minutes as u32 * *TICKS_PER_MINUTE.borrow()) as i64;
    if let Some(item_id) = *CURR_ITEMID.borrow() {
        NOTIFICATIONS_ITEMS
            .borrow_mut()
            .push((item_id, tick, notification));
    } else {
        if !is_entity_dead(*CURR_ENTITYID.borrow()) {
            NOTIFICATIONS_ENTITIES
                .borrow_mut()
                .push((*CURR_ENTITYID.borrow(), tick, notification));
        }
    }
}

/// Returns the name of the sector the entity or item is currently in.
fn get_sector_name() -> String {
    let map = MAP.borrow();

    if let Some(item_id) = *CURR_ITEMID.borrow() {
        for e in map.items.iter() {
            if e.id == item_id {
                let pos = e.get_pos_xz();
                if let Some(s) = map.find_sector_at(pos) {
                    if s.name.is_empty() {
                        return "Unnamed Sector".to_string();
                    } else {
                        return s.name.clone();
                    }
                }
            }
        }
    } else {
        for e in map.entities.iter() {
            if e.id == *CURR_ENTITYID.borrow() {
                let pos = e.get_pos_xz();
                if let Some(s) = map.find_sector_at(pos) {
                    if s.name.is_empty() {
                        return "Unnamed Sector".to_string();
                    } else {
                        return s.name.clone();
                    }
                }
            }
        }
    }

    "Not inside any sector".to_string()
}

/// Faces the entity at a random direction.
fn face_random() {
    let entity_id = *CURR_ENTITYID.borrow();
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.face_random();
    }
}

/// Goto a destination sector with the given speed.
fn goto(destination: String, speed: f32) {
    let mut coord: Option<vek::Vec2<f32>> = None;

    {
        let map = MAP.borrow();
        for sector in &map.sectors {
            if sector.name == destination {
                coord = sector.center(&*map);
            }
        }
    }

    if let Some(coord) = coord {
        let entity_id = *CURR_ENTITYID.borrow();
        if let Some(entity) = MAP
            .borrow_mut()
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
        {
            entity.action = Goto(coord, speed);
        }
    }
}

/// CloseIn: Move within a radius of a target entity with a given speed
fn close_in(target: u32, target_radius: f32, speed: f32) {
    let entity_id = *CURR_ENTITYID.borrow();
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.action = CloseIn(target, target_radius, speed);
    }
}

/// Randomly walks
fn random_walk(
    distance: PyObjectRef,
    speed: PyObjectRef,
    max_sleep: PyObjectRef,
    vm: &VirtualMachine,
) {
    let distance: f32 = get_f32(distance, 1.0, vm);
    let speed: f32 = get_f32(speed, 1.0, vm);
    let max_sleep: i32 = get_i32(max_sleep, 0, vm);

    let entity_id = *CURR_ENTITYID.borrow();
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.action = RandomWalk(distance, speed, max_sleep, 0, zero());
    }
}

/// Randomly walks within the current sector.
fn random_walk_in_sector(
    distance: PyObjectRef,
    speed: PyObjectRef,
    max_sleep: PyObjectRef,
    vm: &VirtualMachine,
) {
    let distance: f32 = get_f32(distance, 1.0, vm); // Default distance: 1.0
    let speed: f32 = get_f32(speed, 1.0, vm); // Default speed: 1.0
    let max_sleep: i32 = get_i32(max_sleep, 0, vm); // Default max_sleep: 0

    let entity_id = *CURR_ENTITYID.borrow();
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.action = RandomWalkInSector(distance, speed, max_sleep, 0, zero());
    }
}

/// Set Proximity Tracking
pub fn set_proximity_tracking(
    args: rustpython_vm::function::FuncArgs,
    vm: &VirtualMachine,
) -> PyResult<()> {
    let mut turn_on = false;
    let mut distance = 5.0;

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::Bool(v)) = Value::from_pyobject(arg.clone(), vm) {
                turn_on = v;
            }
        } else if i == 1 {
            if let Some(Value::Float(v)) = Value::from_pyobject(arg.clone(), vm) {
                distance = v;
            }
        }
    }

    if let Some(item_id) = *CURR_ITEMID.borrow() {
        if turn_on {
            ITEM_PROXIMITY_ALERTS.borrow_mut().insert(item_id, distance);
        } else {
            ITEM_PROXIMITY_ALERTS.borrow_mut().remove(&item_id);
        }
    } else {
        let entity_id = *CURR_ENTITYID.borrow();
        if turn_on {
            ENTITY_PROXIMITY_ALERTS
                .borrow_mut()
                .insert(entity_id, distance);
        } else {
            ENTITY_PROXIMITY_ALERTS.borrow_mut().remove(&entity_id);
        }
    }

    Ok(())
}

/// Teleport
pub fn teleport(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut sector_name = String::new();
    let mut region_name = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                sector_name = v.clone();
            }
        } else if i == 1 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                region_name = v.clone();
            }
        }
    }

    if region_name.is_empty() {
        // Teleport entity in this region to the given sector.

        let mut new_pos: Option<vek::Vec2<f32>> = None;
        let mut map: ref_thread_local::RefMut<'_, Map> = MAP.borrow_mut();
        for sector in &map.sectors {
            if sector.name == sector_name {
                new_pos = sector.center(&*map);
            }
        }

        if let Some(new_pos) = new_pos {
            let entity_id = *CURR_ENTITYID.borrow();
            let mut entities = map.entities.clone();
            if let Some(entity) = entities.iter_mut().find(|entity| entity.id == entity_id) {
                entity.set_pos_xz(new_pos);
                check_player_for_section_change(&*map, entity);
            }
            map.entities = entities;
        }
    } else {
        // Remove the entity from this region and send it to the server to be moved
        // into a new region.

        let mut map = MAP.borrow_mut();
        let entity_id = *CURR_ENTITYID.borrow();
        if let Some(pos) = map.entities.iter().position(|e| e.id == entity_id) {
            let removed = map.entities.remove(pos);

            ENTITY_CLASSES.borrow_mut().remove(&removed.id);

            let msg = RegionMessage::TransferEntity(
                *REGIONID.borrow(),
                removed,
                region_name,
                sector_name,
            );
            FROM_SENDER.borrow().get().unwrap().send(msg).unwrap();
        }
    }

    Ok(())
}

/// Message
pub fn message(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut receiver = None;
    let mut message = None;
    let mut category = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::UInt(v)) = Value::from_pyobject(arg.clone(), vm) {
                receiver = Some(v);
            } else if let Some(Value::Int(v)) = Value::from_pyobject(arg.clone(), vm) {
                receiver = Some(v as u32);
            }
        } else if i == 1 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                message = Some(v);
            }
        } else if i == 2 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                category = v.clone();
            }
        }
    }

    if receiver.is_some() && message.is_some() {
        let mut entity_id = Some(*CURR_ENTITYID.borrow());
        let item_id = *CURR_ITEMID.borrow();
        if item_id.is_some() {
            entity_id = None;
        }

        let message = message.unwrap();
        let msg = RegionMessage::Message(
            *REGIONID.borrow(),
            entity_id,
            item_id,
            receiver.unwrap() as u32,
            message,
            category,
        );
        FROM_SENDER.borrow().get().unwrap().send(msg).unwrap();
    }

    Ok(())
}

/// Debug
pub fn debug(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut output = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        let arg_str = match vm.call_method(arg.as_object(), "__repr__", ()) {
            Ok(repr_obj) => match repr_obj.str(vm) {
                Ok(s) => s.as_str().to_owned(),
                Err(_) => "<error converting repr to str>".to_owned(),
            },
            Err(_) => "<error calling __repr__>".to_owned(),
        };

        if i > 0 {
            output.push(' ');
        }
        output.push_str(&arg_str);
    }

    if let Some(name) = get_attr_internal("name") {
        output = format!("{}: {}", name, output);
    }

    send_log_message(output);
    Ok(())
}

/// Get an i32 config value
fn get_config_i32_default(table: &str, key: &str, default: i32) -> i32 {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_integer() {
                return v as i32;
            }
        }
    }
    default
}

fn _get_config_f32_default(table: &str, key: &str, default: f32) -> f32 {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_float() {
                return v as f32;
            }
        }
    }
    default
}

fn _get_config_bool_default(table: &str, key: &str, default: bool) -> bool {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_bool() {
                return v;
            }
        }
    }
    default
}

fn get_config_string_default(table: &str, key: &str, default: &str) -> String {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_str() {
                return v.to_string();
            }
        }
    }
    default.to_string()
}

/// Get an attribute value from the current item or entity.
fn get_attr_internal(key: &str) -> Option<Value> {
    if let Some(id) = *CURR_ITEMID.borrow() {
        if let Some(item) = MAP.borrow().items.iter().find(|item| item.id == id) {
            return item.attributes.get(key).cloned();
        }
    } else {
        let id = *CURR_ENTITYID.borrow();
        if let Some(entity) = MAP.borrow().entities.iter().find(|entity| entity.id == id) {
            return entity.attributes.get(key).cloned();
        }
    };

    None
}

/// Create a new item with the given class name.
fn create_item(class_name: String) -> Option<Item> {
    if !ASSETS.borrow().items.contains_key(&class_name) {
        return None;
    }

    let id = get_global_id();
    let mut item = Item {
        id,
        ..Default::default()
    };

    item.set_attribute("class_name", Value::Str(class_name.clone()));
    item.set_attribute("name", Value::Str(class_name.clone()));

    // Setting the data for the item.
    if let Some(data) = ITEM_CLASS_DATA.borrow().get(&class_name) {
        apply_item_data(&mut item, data);
    }

    // Send active state
    let cmd = format!(
        "{}.event(\"active\", {})",
        class_name,
        if item.attributes.get_bool_default("active", false) {
            "True"
        } else {
            "False"
        }
    );
    TO_EXECUTE_ITEM
        .borrow_mut()
        .push((item.id, "active".into(), cmd));

    Some(item)
}

/// Create a new entity instance.
pub fn create_entity_instance(mut entity: Entity) {
    entity.id = get_global_id();
    entity.set_attribute(
        "_source_seq",
        Value::Source(PixelSource::Sequence("idle".into())),
    );
    entity.set_attribute("mode", Value::Str("active".into()));
    entity.mark_all_dirty();
    MAP.borrow_mut().entities.push(entity.clone());

    let name = MAP.borrow().name.clone();

    // Send "startup" event
    if let Some(class_name) = entity.get_attr_string("class_name") {
        // Setting the data for the entity
        if let Some(data) = ENTITY_CLASS_DATA.borrow().get(&class_name) {
            let mut map = MAP.borrow_mut();
            for e in map.entities.iter_mut() {
                if e.id == entity.id {
                    apply_entity_data(e, data);

                    // Fill up the inventory slots
                    if let Some(Value::Int(inv_slots)) = e.attributes.get("inventory_slots") {
                        e.inventory = vec![];
                        for _ in 0..*inv_slots {
                            e.inventory.push(None);
                        }
                    }
                }
            }
        }

        *CURR_ENTITYID.borrow_mut() = entity.id;

        // Register player
        if ENTITY_PLAYER_CLASSES.borrow().contains(&class_name) {
            register_player()
        }

        let cmd = format!("{}.event(\"startup\", \"\")", class_name);
        ENTITY_CLASSES
            .borrow_mut()
            .insert(entity.id, class_name.clone());
        if let Err(err) = REGION.borrow_mut().execute(&cmd) {
            send_log_message(format!(
                "{}: Event Error ({}) for '{}': {}",
                name,
                "startup",
                get_entity_name(entity.id),
                err,
            ));
        }

        // Determine, set and notify the entity about the sector it is in.
        let mut sector_name = String::new();
        if let Some(sector) = MAP.borrow().find_sector_at(entity.get_pos_xz()) {
            sector_name = sector.name.clone();
        }
        {
            let mut map = MAP.borrow_mut();
            for e in map.entities.iter_mut() {
                if e.id == entity.id {
                    e.attributes.set("sector", Value::Str(sector_name.clone()));
                }
            }
        }
        if !sector_name.is_empty() {
            let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector_name);
            _ = REGION.borrow_mut().execute(&cmd);
        }
    }

    // Running the character setup script
    if let Some(setup) = entity.get_attr_string("setup") {
        if let Err(err) = REGION.borrow_mut().execute(&setup) {
            send_log_message(format!(
                "{}: Setup '{}/{}': {}",
                name,
                entity.get_attr_string("name").unwrap_or("Unknown".into()),
                entity
                    .get_attr_string("class_name")
                    .unwrap_or("Unknown".into()),
                err,
            ));
            *ERROR_COUNT.borrow_mut() += 1;
        }

        *CURR_ENTITYID.borrow_mut() = entity.id;
        if let Err(err) = REGION.borrow_mut().execute("setup()") {
            send_log_message(format!(
                "{}: Setup '{}/{}': {}",
                name,
                entity.get_attr_string("name").unwrap_or("Unknown".into()),
                entity
                    .get_attr_string("class_name")
                    .unwrap_or("Unknown".into()),
                err,
            ));
            *ERROR_COUNT.borrow_mut() += 1;
        }
    }

    send_log_message(format!(
        "{}: Spawned `{}`",
        name,
        get_entity_name(entity.id),
    ));
}

/// Received an entity from another region
pub fn receive_entity(mut entity: Entity, dest_sector_name: String) {
    entity.action = EntityAction::Off;

    let mut map = MAP.borrow_mut();
    let mut entities = map.entities.clone();

    let mut new_pos: Option<vek::Vec2<f32>> = None;
    for sector in &map.sectors {
        if sector.name == dest_sector_name {
            new_pos = sector.center(&*map);
        }
    }

    if let Some(new_pos) = new_pos {
        entity.set_pos_xz(new_pos);
        check_player_for_section_change(&*map, &mut entity);
    }

    if let Some(class_name) = entity.get_attr_string("class_name") {
        ENTITY_CLASSES
            .borrow_mut()
            .insert(entity.id, class_name.clone());
    }

    entities.push(entity);
    map.entities = entities;
}

fn id() -> u32 {
    *CURR_ENTITYID.borrow()
}
