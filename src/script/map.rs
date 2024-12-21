use crate::script::ParseError;
use crate::Map;
use rustpython::vm;
use rustpython::vm::*;
use std::sync::{LazyLock, RwLock};
use vek::Vec2;

static MAP: LazyLock<RwLock<Map>> = LazyLock::new(|| RwLock::new(Map::default()));
static POSITION: LazyLock<RwLock<Vec2<f32>>> = LazyLock::new(|| RwLock::new(Vec2::zero()));

fn wall(length: f32) -> PyResult<()> {
    // println!(
    //     "Added room with width={}, height={}, floor={}, ceiling={}",
    //     width, height, floor, ceiling
    // );

    let mut map = MAP.write().unwrap();

    let from = POSITION.read().unwrap();
    let to = Vec2::new(from.x + length, from.y);

    let from_index = map.add_vertex_at(from.x, from.y);
    let to_index = map.add_vertex_at(to.x, to.y);

    map.create_linedef(from_index, to_index);

    Ok(())
}

pub struct MapScript {
    error: Option<ParseError>,
}

impl Default for MapScript {
    fn default() -> Self {
        MapScript::new()
    }
}

impl MapScript {
    pub fn new() -> Self {
        Self { error: None }
    }

    /// Parse the source and return a valid map.
    pub fn run(&mut self, source: String) -> Result<Map, Vec<String>> {
        self.error = None;
        *MAP.write().unwrap() = Map::default();

        let interpreter = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        interpreter.enter(|vm| {
            let scope = vm.new_scope_with_builtins();
            // vm.add_frozen(py_freeze!(
            //     source = "def foo(): pass",
            //     module_name = "otherthing"
            // ));

            let _ = scope
                .globals
                .set_item("wall", vm.new_function("wall", wall).into(), vm);

            if let Ok(code_obj) = vm
                .compile(&source, vm::compiler::Mode::Exec, "<embedded>".to_owned())
                .map_err(|err| vm.new_syntax_error(&err, Some(&source)))
            {
                if let Err(err) = vm.run_code_obj(code_obj, scope) {
                    let args = err.args();

                    let mut errors: Vec<String> = vec![];
                    for error in args.iter() {
                        if let Ok(msg) = error.str(vm) {
                            errors.push(msg.to_string());
                        }
                    }

                    return Err(errors);
                }
            }

            Ok(MAP.read().unwrap().clone())
        })
    }
}
