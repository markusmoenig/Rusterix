use crate::server::py_fn::*;
use crate::{Assets, Entity, EntityAction, Map, MapMini, PixelSource, PlayerCamera, Value};
use crossbeam_channel::{select, tick, unbounded, Receiver, Sender};
use rand::*;
use ref_thread_local::{ref_thread_local, RefThreadLocal};

use rustpython::vm::*;
use std::sync::{Arc, Mutex, OnceLock};
use theframework::prelude::{FxHashMap, FxHashSet, TheTime, Uuid};
use vek::num_traits::zero;

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

    /// Id counter.
    pub static managed ID_GEN: u32 = 0;

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

    /// Maps an entity class name to its data file.
    pub static managed ENTITY_CLASS_DATA: FxHashMap<String, String> = FxHashMap::default();

    /// Maps an item class name to its data file.
    pub static managed ITEM_CLASS_DATA: FxHashMap<String, String> = FxHashMap::default();

    /// Cmds which are queued to be executed to either entities or items.
    pub static managed TO_EXECUTE_ENTITY: Vec<(u32, String)> = vec![];
    pub static managed TO_EXECUTE_ITEM  : Vec<(u32, String)> = vec![];

    /// Errors since starting the region.
    pub static managed ERROR_COUNT: u32 = 0;
    pub static managed STARTUP_ERRORS: Vec<String> = vec![];

    pub static managed TO_RECEIVER: OnceLock<Receiver<RegionMessage>> = OnceLock::new();
    pub static managed FROM_SENDER: OnceLock<Sender<RegionMessage>> = OnceLock::new();
}

use super::data::{apply_entity_data, apply_item_data};
use super::RegionMessage;
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
                "set_attr",
                vm.new_function("set_attr", set_attr).into(),
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

            let _ = scope
                .globals
                .set_item("debug", vm.new_function("debug", debug).into(), vm);

            let _ = scope.globals.set_item(
                "entities_in_radius",
                vm.new_function("entities_in_radius", entities_in_radius)
                    .into(),
                vm,
            );
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
    pub fn init(&mut self, name: String, map: Map, assets: &Assets) {
        self.name = name;

        *ID_GEN.borrow_mut() = 0;
        *MAP.borrow_mut() = map;
        *NOTIFICATIONS_ENTITIES.borrow_mut() = vec![];
        *NOTIFICATIONS_ITEMS.borrow_mut() = vec![];
        *STARTUP_ERRORS.borrow_mut() = vec![];
        *BLOCKING_TILES.borrow_mut() = assets.blocking_tiles();

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

        // Set an entity id and mark all fields dirty for the first transmission to the server.
        for e in MAP.borrow_mut().entities.iter_mut() {
            e.id = *ID_GEN.borrow();
            *ID_GEN.borrow_mut() += 1;
            e.mark_all_dirty();
        }

        // Set an item id and mark all fields dirty for the first transmission to the server.
        for i in MAP.borrow_mut().items.iter_mut() {
            i.id = *ID_GEN.borrow();
            *ID_GEN.borrow_mut() += 1;
            i.mark_all_dirty();
        }
    }

    /// Run this instance
    pub fn run(self) {
        let system_ticker = tick(std::time::Duration::from_millis(250));
        let redraw_ticker = tick(std::time::Duration::from_millis(16));

        // We have to reassign map inside the thread
        let map = MAP.borrow_mut().clone();
        let name = map.name.clone();
        let startup_errors = STARTUP_ERRORS.borrow().clone();
        let entity_class_data = ENTITY_CLASS_DATA.borrow().clone();
        let item_class_data = ITEM_CLASS_DATA.borrow().clone();
        let blocking_tiles = BLOCKING_TILES.borrow().clone();

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
            // TODO: Make this configurable
            *TICKS_PER_MINUTE.borrow_mut() = 4;
            *ENTITY_CLASS_DATA.borrow_mut() = entity_class_data;
            *ITEM_CLASS_DATA.borrow_mut() = item_class_data;
            *BLOCKING_TILES.borrow_mut() = blocking_tiles;

            // Send startup messages
            *ERROR_COUNT.borrow_mut() = startup_errors.len() as u32;
            for l in startup_errors {
                send_log_message(l);
            }

            // Send "startup" event to all entities.
            let entities = MAP.borrow().entities.clone();
            for entity in entities.iter() {
                if let Some(class_name) = entity.get_attr_string("class_name") {
                    let cmd = format!("{}.event(\"startup\", \"\")", class_name);
                    ENTITY_CLASSES.borrow_mut().insert(entity.id, class_name);
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
                }
            }

            // Send "startup" event to all items.
            let items = MAP.borrow().items.clone();
            for item in items.iter() {
                if let Some(class_name) = item.get_attr_string("class_name") {
                    let cmd = format!("{}.event(\"startup\", \"\")", class_name);
                    ITEM_CLASSES.borrow_mut().insert(item.id, class_name);
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

                    // Setting the data for the entity.
                    if let Some(class_name) = entity.get_attr_string("class_name") {
                        if let Some(data) = ENTITY_CLASS_DATA.borrow().get(&class_name) {
                            let mut map = MAP.borrow_mut();
                            for e in map.entities.iter_mut() {
                                if e.id == entity.id {
                                    apply_entity_data(e, data);
                                }
                            }
                        }
                    }
                }
            }

            // Running the item setup scripts for the class instances
            let items = MAP.borrow().items.clone();
            for item in items.iter() {
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
                            }
                        }
                    }
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
                                if let Some(class_name) = ENTITY_CLASSES.borrow().get(id) {
                                    let cmd = format!("{}.event(\"{}\", \"\")", class_name, notification);
                                    *CURR_ENTITYID.borrow_mut() = *id;
                                    *CURR_ITEMID.borrow_mut() = None;
                                    let _ = REGION.borrow_mut().execute(&cmd);
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
                    }
                    recv(redraw_ticker) -> _ => {
                        REGION.borrow_mut().handle_redraw_tick();

                        // Execute delayed scripts for entities
                        // This is because we can only borrow REGION once.
                        let to_execute_entity = TO_EXECUTE_ENTITY.borrow().clone();
                        TO_EXECUTE_ENTITY.borrow_mut().clear();
                        for todo in to_execute_entity {
                            *CURR_ENTITYID.borrow_mut() = todo.0;
                            *CURR_ITEMID.borrow_mut() = None;
                            if let Err(err) = REGION.borrow().execute(&todo.1) {
                                println!("err {}", err);
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
                            if let Err(err) = REGION.borrow().execute(&todo.1) {
                                println!("err {}", err);
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
    fn handle_redraw_tick(&mut self) {
        let mut updates: Vec<Vec<u8>> = vec![];
        let mut item_updates: Vec<Vec<u8>> = vec![];
        let mut entities = MAP.borrow().entities.clone();

        for entity in &mut entities {
            match &entity.action {
                EntityAction::Forward => {
                    if entity.is_player() {
                        if let Some(Value::PlayerCamera(player_camera)) =
                            entity.attributes.get("player_camera")
                        {
                            if *player_camera != PlayerCamera::D3FirstP {
                                entity.face_north();
                            }
                            self.move_entity(entity, 1.0);
                        }
                    } else {
                        self.move_entity(entity, 1.0);
                    }
                }
                EntityAction::Left => {
                    if entity.is_player() {
                        if let Some(Value::PlayerCamera(player_camera)) =
                            entity.attributes.get("player_camera")
                        {
                            if *player_camera != PlayerCamera::D3FirstP {
                                entity.face_west();
                                self.move_entity(entity, 1.0);
                            } else {
                                entity.turn_left(2.0);
                            }
                        }
                    } else {
                        entity.turn_left(2.0);
                    }
                }
                EntityAction::Right => {
                    if entity.is_player() {
                        if let Some(Value::PlayerCamera(player_camera)) =
                            entity.attributes.get("player_camera")
                        {
                            if *player_camera != PlayerCamera::D3FirstP {
                                entity.face_east();
                                self.move_entity(entity, 1.0);
                            } else {
                                entity.turn_right(2.0);
                            }
                        }
                    } else {
                        entity.turn_right(2.0);
                    }
                }
                EntityAction::Backward => {
                    if entity.is_player() {
                        if let Some(Value::PlayerCamera(player_camera)) =
                            entity.attributes.get("player_camera")
                        {
                            if *player_camera != PlayerCamera::D3FirstP {
                                entity.face_south();
                                self.move_entity(entity, 1.0);
                            } else {
                                self.move_entity(entity, -1.0);
                            }
                        }
                    } else {
                        self.move_entity(entity, -1.0);
                    }
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
                            let mut rng = rand::thread_rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.gen_range(0..=*max_sleep) as u32,
                                RandomWalk(*distance, *speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let t = RandomWalk(*distance, *speed, *max_sleep, 0, *target);
                            let max_sleep = *max_sleep;
                            let blocked = self.move_entity(entity, 1.0);
                            if blocked {
                                let mut rng = rand::thread_rng();
                                entity.action = self.create_sleep_switch_action(
                                    rng.gen_range(0..=max_sleep) as u32,
                                    t,
                                );
                            }
                        }
                    }
                }
                EntityAction::RandomWalkInSector(speed, max_sleep, state, target) => {
                    if *state == 0 {
                        // State 0: Uninitialized, find a target location.
                        let map = MAP.borrow();
                        if let Some(sector) = map.find_sector_at(entity.get_pos_xz()) {
                            if let Some(pos) = sector.get_random_position(&map) {
                                entity.action = RandomWalkInSector(*speed, *max_sleep, 1, pos);
                                entity.face_at(pos);
                            }
                        }
                    } else if *state == 1 {
                        // State 1: Walk towards
                        if target.distance(entity.get_pos_xz()) < 0.1 {
                            // Arrived, Sleep
                            let mut rng = rand::thread_rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.gen_range(0..=*max_sleep) as u32,
                                RandomWalkInSector(*speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let t = RandomWalkInSector(*speed, *max_sleep, 0, *target);
                            let max_sleep = *max_sleep;
                            let blocked = self.move_entity(entity, 1.0);
                            if blocked {
                                let mut rng = rand::thread_rng();
                                entity.action = self.create_sleep_switch_action(
                                    rng.gen_range(0..=max_sleep) as u32,
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

                    if let Some(tb) = error.traceback() {
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

    /// Moves an entity forward or backward. Returns true if blocked.
    fn move_entity(&self, entity: &mut Entity, dir: f32) -> bool {
        let speed = 0.05 * 2.0;
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
                        TO_EXECUTE_ENTITY.borrow_mut().push((entity.id, cmd));
                    }
                    // Send "bumped_by_entity" for the other entity
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&other.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_by_entity", entity.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((other.id, cmd));
                    }
                    // if the other entity is blocking, stop the movement
                    if other.attributes.get_bool_default("blocking", false) {
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
                    println!("hit");
                    if let Some(class_name) = ENTITY_CLASSES.borrow().get(&entity.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_into_item", other.id
                        );
                        TO_EXECUTE_ENTITY.borrow_mut().push((entity.id, cmd));
                    }
                    if let Some(class_name) = ITEM_CLASSES.borrow().get(&other.id) {
                        let cmd = format!(
                            "{}.event('{}', {})",
                            class_name, "bumped_by_entity", entity.id
                        );
                        TO_EXECUTE_ITEM.borrow_mut().push((other.id, cmd));
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
    }
}

/// Register Player
fn register_player() {
    let region_id = *REGIONID.borrow();
    let entity_id = *CURR_ENTITYID.borrow();

    if let Some(entity) = MAP.borrow_mut().entities.get_mut(entity_id as usize) {
        entity.set_attribute("is_player", Value::Bool(true));
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

    if let Some(entity) = MAP.borrow_mut().entities.get_mut(entity_id as usize) {
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
        if let Some(entity) = MAP
            .borrow_mut()
            .entities
            .get_mut(*CURR_ENTITYID.borrow() as usize)
        {
            entity.action = parsed_action;
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

    // Convert Vec<u32> to a Python list using `vm.ctx.new_list()`
    let py_list = vm.ctx.new_list(
        entities
            .iter()
            .map(|&id| vm.ctx.new_int(id).into()) // Convert `PyRef<PyInt>` to `PyObjectRef`
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Notify the entity / item in the given amount of minutes.
fn notify_in(minutes: i32, notification: String) {
    let tick = *TICKS.borrow() + (minutes as u32 * *TICKS_PER_MINUTE.borrow()) as i64;
    if let Some(item_id) = *CURR_ITEMID.borrow() {
        NOTIFICATIONS_ITEMS
            .borrow_mut()
            .push((item_id, tick, notification));
    } else {
        NOTIFICATIONS_ENTITIES
            .borrow_mut()
            .push((*CURR_ENTITYID.borrow(), tick, notification));
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
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .get_mut(*CURR_ENTITYID.borrow() as usize)
    {
        entity.face_random();
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

    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .get_mut(*CURR_ENTITYID.borrow() as usize)
    {
        entity.action = RandomWalk(distance, speed, max_sleep, 0, zero());
    }
}

/// Randomly walks within the current sector.
fn random_walk_in_sector(speed: PyObjectRef, max_sleep: PyObjectRef, vm: &VirtualMachine) {
    let speed: f32 = get_f32(speed, 1.0, vm); // Default speed: 1.0
    let max_sleep: i32 = get_i32(max_sleep, 0, vm); // Default max_sleep: 0

    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .get_mut(*CURR_ENTITYID.borrow() as usize)
    {
        entity.action = RandomWalkInSector(speed, max_sleep, 0, zero());
    }
}

/// Debug
pub fn debug(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut output = String::new();
    for (i, arg) in args.args.iter().enumerate() {
        // Convert each argument to a string using Python's `str()` method
        let arg_str = match arg.str(vm) {
            Ok(s) => s.as_str().to_owned(),
            Err(_) => "<error converting to string>".to_owned(),
        };
        if i > 0 {
            output.push(' ');
        }
        output.push_str(&arg_str);
    }

    send_log_message(output);
    Ok(())
}
