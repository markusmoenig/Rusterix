use rustpython::vm::{Interpreter, PyResult};
use std::sync::{Arc, Mutex};

struct RegionInstance {
    interp: Interpreter,
    scope: Arc<Mutex<rustpython_vm::scope::Scope>>,
}

impl RegionInstance {
    pub fn new() -> Self {
        let interp = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        let scope = Arc::new(Mutex::new(interp.enter(|vm| vm.new_scope_with_builtins())));
        Self { interp, scope }
    }

    pub fn init(&mut self) {
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

        let _ = self.execute("manager = EntityManager()");
    }

    pub fn add_entity(&mut self, name: String) {
        // let cmd = format!("manager.create_entity(Entity())", name);
        let cmd = format!(
            "entity = Entity(); entity.attributes['name'] = '{}'; manager.add_entity(entity);",
            name
        );
        let _ = self.execute(&cmd);
    }

    pub fn execute(&mut self, source: &str) -> PyResult {
        let scope = self.scope.lock().unwrap();

        self.interp
            .enter(|vm| vm.run_code_string(scope.clone(), source, "".to_string()))
    }
}

/*
pub fn get_error(&self, error: PyRef<PyBaseException>) -> String {
    let args = err.args();

    if let Some(err) = args.first() {
        if let Ok(msg) = err.str(vm) {
            return msg.to_string();
        }
    }

    return "".into();
}*/

pub fn py_test() {
    let mut inst = RegionInstance::new();
    inst.init();
    inst.add_entity("Markus".to_string());
    let _ = inst.execute("manager.debug()");
}
