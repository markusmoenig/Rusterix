use crate::{Assets, EntityAction, Value};
use rustpython::vm::*;
use std::{
    str::FromStr,
    sync::{Arc, LazyLock, Mutex, RwLock},
};

pub static ACTIONCMD: LazyLock<RwLock<EntityAction>> =
    LazyLock::new(|| RwLock::new(EntityAction::Off));

fn action(action_str: String) {
    if let Ok(action) = EntityAction::from_str(&action_str) {
        *ACTIONCMD.write().unwrap() = action;
    }
}

pub struct ClientAction {
    interp: Interpreter,
    scope: Arc<Mutex<rustpython_vm::scope::Scope>>,
    class_name: String,
}

impl Default for ClientAction {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientAction {
    pub fn new() -> Self {
        let interp = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        let scope = Arc::new(Mutex::new(interp.enter(|vm| vm.new_scope_with_builtins())));

        interp.enter(|vm| {
            let scope = scope.lock().unwrap();

            let _ = scope
                .globals
                .set_item("action", vm.new_function("action", action).into(), vm);
        });

        Self {
            interp,
            scope,
            class_name: String::new(),
        }
    }

    /// Init
    pub fn init(&mut self, class_name: String, assets: &Assets) {
        if let Some((entity_source, _)) = assets.entities.get(&class_name) {
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
            }
            self.class_name = class_name;
        }
    }

    /// Execute the user event
    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        let cmd = format!("{}.user_event('{}', '{}')", self.class_name, event, value);
        if let Err(err) = self.execute(&cmd) {
            println!("Client: Error {} User Event: {}", self.class_name, err,);
        }

        ACTIONCMD.read().unwrap().clone()
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
}
