use crate::Map;
use rustpython::vm::{Interpreter, PyObjectRef};
use std::sync::{Arc, Mutex};
use theframework::prelude::FxHashMap;

pub struct Region {
    interp: Interpreter,
    scope: Arc<Mutex<rustpython_vm::scope::Scope>>,

    name: String,
    map: Map,
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
        Self {
            interp,
            scope,

            name: String::new(),
            map: Map::default(),
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
        let _ = self.execute("manager = EntityManager()");

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

        // Installing Entity Instances
        for entity in &self.map.entities.clone() {
            let cmd = format!(
                "entity = {}(None, None, None); result = manager.add_entity(entity);",
                entity.class_name
            );
            match self.execute(&cmd) {
                Ok(_obj) => {
                    self.interp.enter(|vm| {
                        if let Ok(result) =
                            self.scope.lock().unwrap().globals.get_item("result", vm)
                        {
                            if let Ok(value) = result.try_into_value::<i32>(vm) {
                                println!(
                                    "Initialized {}/{} ({}) to '{}': Ok",
                                    entity.name, entity.class_name, value, self.name
                                );
                            }
                        }
                    });
                }
                Err(err) => {
                    println!("Error for {}/{}: {}", entity.name, entity.class_name, err);
                }
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

    pub fn execute(&mut self, source: &str) -> Result<PyObjectRef, String> {
        let scope = self.scope.lock().unwrap();

        self.interp.enter(|vm| {
            let rc = vm.run_code_string(scope.clone(), source, "".to_string());
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
