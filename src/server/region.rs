use crate::server::{player_action, register_player};
use crate::{Entity, EntityAction, Map};
use crossbeam_channel::{select, tick, unbounded, Receiver, Sender};
use rustpython::vm::{Interpreter, PyObjectRef};
use std::sync::{Arc, Mutex};
use theframework::prelude::FxHashMap;

use super::RegionMessage;
use vek::Vec3;
// use EntityAction::*;

use RegionMessage::*;

pub struct Region {
    pub id: u32,

    interp: Interpreter,
    scope: Arc<Mutex<rustpython_vm::scope::Scope>>,

    name: String,
    map: Map,

    /// The registered, local player for this entity
    player_id: Option<u32>,

    /// Send messages to this region
    pub to_sender: Sender<RegionMessage>,
    /// Local receiver
    to_receiver: Receiver<RegionMessage>,

    /// Send messages from this region
    from_sender: Sender<RegionMessage>,
    /// Local receiver
    pub from_receiver: Receiver<RegionMessage>,
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

impl Region {
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
                "action",
                vm.new_function("action", player_action).into(),
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
            map: Map::default(),

            player_id: None,

            to_receiver,
            to_sender,
            from_receiver,
            from_sender,
        }
    }

    /// Initializes the Python bases classes, sets the map and applies entities
    pub fn init(&mut self, name: String, map: Map, entities: &FxHashMap<String, String>) {
        self.name = name;
        self.map = map;

        // Apply the base classes
        if let Some(bytes) = crate::Embedded::get("entity.py") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                let _ = self.execute(source);
            }
        }
        if let Some(bytes) = crate::Embedded::get("entitymanager.py") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                let _ = self.execute(source);
            }
        }

        // Create the manager
        let _ = self.execute(&format!("manager = EntityManager({})", self.id));

        // Installing Entity Class
        for (name, entity_source) in entities {
            match self.execute(entity_source) {
                Ok(_) => {
                    println!("Installing {} Class to '{}': Ok", name, self.name);
                }
                Err(_err) => {
                    println!("Installing {} Class to '{}': Error", name, self.name);
                }
            }
        }

        let entities = self.map.entities.clone();

        // Installing Entity Instances
        for (index, entity) in entities.iter().enumerate() {
            let cmd = format!(
                "manager.add_entity({}([{}, {}, {}], None, None))",
                entity.class_name, entity.position.x, entity.position.y, entity.position.z,
            );
            match self.execute(&cmd) {
                Ok(obj) => {
                    self.interp.enter(|vm| {
                        if let Ok(value) = obj.try_into_value::<i32>(vm) {
                            println!(
                                "Initialized {}/{} ({}) to '{}': Ok",
                                entity.name, entity.class_name, value, self.name
                            );
                            if let Some(e) = self.map.entities.get_mut(index) {
                                e.id = value as u32;
                            }
                        }
                    });
                }
                Err(err) => {
                    println!("Error for {}/{}: {}", entity.name, entity.class_name, err);
                }
            }
        }

        // for entity in &self.map.entities.clone() {
        //     self.set_entity_position(entity.id, Vec3::new(1.0, 2.0, 3.0));
        // }
    }

    pub fn run(mut self) {
        //let ticker = tick(std::time::Duration::from_millis(250));
        let ticker = tick(std::time::Duration::from_millis(16));

        std::thread::spawn(move || {
            loop {
                select! {
                    recv(ticker) -> _ => {
                        //_ = self.tick()
                        for entity in &mut self.map.entities {
                            if Some(entity.id) == self.player_id {
                                match entity.action {
                                    EntityAction::North => {
                                        entity.move_forward(0.05 * 2.0);
                                        self.from_sender.send(RegionMessage::Entity(entity.clone())).unwrap();
                                    }
                                    EntityAction::West => {
                                        entity.turn_left(2.0);
                                        self.from_sender.send(RegionMessage::Entity(entity.clone())).unwrap();
                                    }
                                    EntityAction::East => {
                                        entity.turn_right(2.0);
                                        self.from_sender.send(RegionMessage::Entity(entity.clone())).unwrap();
                                    }
                                    EntityAction::South => {
                                        entity.move_backward(0.05 * 2.0);
                                        self.from_sender.send(RegionMessage::Entity(entity.clone())).unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    },
                    recv(self.to_receiver) -> mess => {
                        if let Ok(message) = mess {
                            match message {
                                RegisterPlayer(entity_id) => {
                                    println!(
                                        "Region {} ({}): Registering player {}",
                                        self.name, self.id, entity_id
                                    );
                                    if let Some(entity) = self.map.entities.get(entity_id as usize) {
                                        self.from_sender.send(RegionMessage::Entity(entity.clone())).unwrap();
                                    }
                                    self.player_id = Some(entity_id);
                                }
                                Event(entity_id, event, value) => {
                                    let cmd = format!("manager.event({}, '{}', {})", entity_id, event, value);
                                    match self.execute(&cmd) {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!("Event error: {} in {}", err, cmd);
                                        }
                                    }
                                }
                                UserEvent(entity_id, event, value) => {
                                    let cmd = format!("manager.user_event({}, '{}', {})", entity_id, event, value);
                                    match self.execute(&cmd) {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!("User event error: {} in {}", err, cmd);
                                        }
                                    }
                                }
                                UserAction(entity_id, action) => {
                                    if let Some(entity) = self.get_entity_mut(entity_id) {
                                        entity.action = action;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        });
    }

    /// Return the Rust side of the Entity for the given Id
    pub fn get_entity(&self, id: u32) -> Option<&Entity> {
        self.map.entities.iter().find(|e| e.id == id)
    }

    /// Return the Rust mut side of the Entity for the given Id
    pub fn get_entity_mut(&mut self, id: u32) -> Option<&mut Entity> {
        self.map.entities.iter_mut().find(|e| e.id == id)
    }

    /// Get the position of the entity of the given id.
    pub fn get_entity_position(&self, id: u32) -> Option<[f32; 3]> {
        let cmd = format!("manager.get_entity_position({})", id);
        match self.execute(&cmd) {
            Ok(obj) => self.interp.enter(|vm| {
                if let Ok(value) = obj.try_into_value::<Vec<f32>>(vm) {
                    Some([value[0], value[1], value[2]])
                } else {
                    None
                }
            }),
            Err(err) => {
                println!("Error getting entity ({}) position: {}", id, err);
                None
            }
        }
    }

    /// Get the position of the entity of the given id.
    pub fn set_entity_position(&self, id: u32, position: Vec3<f32>) {
        let cmd = format!(
            "manager.set_entity_position({}, [{:.3}, {:.3}, {:.3}])",
            id, position.x, position.y, position.z
        );
        match self.execute(&cmd) {
            Ok(_obj) => {}
            Err(err) => {
                println!("Error setting entity ({}) position: {}", id, err);
            }
        }
    }

    pub fn add_entity(&mut self, name: String) {
        // let cmd = format!("manager.create_entity(Entity())", name);
        let cmd = format!(
            "entity = Entity(); entity.attributes['name'] = '{}'; manager.add_entity(entity);",
            name
        );
        let _ = self.execute(&cmd);
    }

    pub fn execute(&self, source: &str) -> Result<PyObjectRef, String> {
        let scope = self.scope.lock().unwrap();

        self.interp.enter(|vm| {
            let rc = vm.run_block_expr(scope.clone(), source);
            match rc {
                Ok(obj) => Ok(obj),
                Err(error) => {
                    let args = error.args();
                    let mut err_string = String::new();
                    if let Some(err) = args.first() {
                        if let Ok(msg) = err.str(vm) {
                            err_string = msg.to_string();
                        }
                    }
                    Err(err_string)
                }
            }
        })
    }

    // pub fn get_error(&self, error: PyRef<PyBaseException>) -> String {
    //     let args = error.args();

    //     if let Some(err) = args.first() {
    //         if let Ok(msg) = err.str(vm) {
    //             return msg.to_string();
    //         }
    //     }

    //     return "".into();
    // }
}

// pub fn py_test() {
//     let mut inst = Region::new();
//     inst.init();
//     inst.add_entity("Markus".to_string());
//     let _ = inst.execute("manager.debug()");
// }
