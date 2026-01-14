use crate::vm::node::hosthandler::HostHandler;
use crate::vm::*;
use crate::{Assets, EntityAction, Value};
use rustpython::vm::*;
use std::str::FromStr;

#[derive(Default)]
struct ClientHostHandler {
    pub action: Option<EntityAction>,
}

impl HostHandler for ClientHostHandler {
    fn on_action(&mut self, v: &VMValue) {
        if let Some(s) = v.as_string() {
            if let Ok(parsed) = EntityAction::from_str(s) {
                self.action = Some(parsed);
            }
        }
    }

    fn on_intent(&mut self, v: &VMValue) {
        if let Some(s) = v.as_string() {
            self.action = Some(EntityAction::Intent(s.to_string()));
        }
    }
}

/// Set the current debug location in the grid.
fn _set_debug_loc(_event: String, _x: u32, _y: u32, _vm: &VirtualMachine) {}

pub struct ClientAction {
    // interp: Interpreter,
    // scope: Arc<Mutex<rustpython_vm::scope::Scope>>,
    vm: VM,
    class_name: String,
    exec: Execution,
}

impl Default for ClientAction {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientAction {
    pub fn new() -> Self {
        /*
        let interp = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        let scope = Arc::new(Mutex::new(interp.enter(|vm| vm.new_scope_with_builtins())));

        interp.enter(|vm| {
            let scope = scope.lock().unwrap();

            let _ = scope
                .globals
                .set_item("action", vm.new_function("action", action).into(), vm);

            let _ = scope
                .globals
                .set_item("intent", vm.new_function("intent", intent).into(), vm);

            let _ = scope.globals.set_item(
                "set_debug_loc",
                vm.new_function("set_debug_loc", set_debug_loc).into(),
                vm,
            );
        });
        */

        Self {
            // interp,
            // scope,
            vm: VM::default(),
            class_name: String::new(),
            exec: Execution::new(0),
        }
    }

    /// Init
    pub fn init(&mut self, class_name: String, assets: &Assets) {
        if let Some((_entity_source, _)) = assets.entities.get(&class_name) {
            /*
            if let Err(err) = self.execute(entity_source) {
                println!(
                    "Client: Error Compiling {} Character Class: {}",
                    class_name, err,
                );
            }
            if let Err(err) = self.execute(&format!("{} = {}()", class_name, class_name)) {
                println!(
                    "Client: Error Installing {} Character Class: {}",
                    class_name, err,
                );
            }*/

            let _result = self.vm.prepare_str(
                r#"
                fn user_event(event, value) {
                    match event {
                        "key_down" {
                            if value == "w" {
                                action("forward");
                            }
                            if value == "a" {
                                action("left");
                            }
                            if value == "d" {
                                action("right");
                            }
                            if value == "s" {
                                action("backward");
                            }
                        }
                        "key_up" {
                            action("none");
                        }
                        _ {}
                    }
                }
                "#,
            );
            match _result {
                Ok(_) => self.exec.reset(self.vm.context.globals.len()),
                Err(e) => eprintln!("Client: error compiling user_event: {}", e),
            }
            self.class_name = class_name;
        }
    }

    /// Execute the user event
    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        if let Some(index) = self
            .vm
            .context
            .program
            .user_functions_name_map
            .get("user_event")
            .copied()
        {
            self.exec.reset(self.vm.context.globals.len());
            let mut handler = ClientHostHandler::default();
            let args = [VMValue::from_string(event), VMValue::from_value(&value)];
            let _ = self.exec.execute_function_host(
                &args,
                index,
                &self.vm.context.program,
                &mut handler,
            );

            if let Some(act) = handler.action {
                return act;
            }
        }

        EntityAction::Off
    }
}
