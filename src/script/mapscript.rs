use super::load_texture;
use crate::script::ParseError;
use crate::{Map, MapMeta, Tile};
use rustpython::vm;
use rustpython::vm::*;
use std::sync::{LazyLock, RwLock};
use theframework::prelude::*;
use vek::Vec2;

static DEFAULT_WALL_TEXTURE: LazyLock<RwLock<Option<Uuid>>> = LazyLock::new(|| RwLock::new(None));
static DEFAULT_FLOOR_TEXTURE: LazyLock<RwLock<Option<Uuid>>> = LazyLock::new(|| RwLock::new(None));
static DEFAULT_CEILING_TEXTURE: LazyLock<RwLock<Option<Uuid>>> =
    LazyLock::new(|| RwLock::new(None));

static MAP: LazyLock<RwLock<Map>> = LazyLock::new(|| RwLock::new(Map::default()));
static TILES: LazyLock<RwLock<FxHashMap<Uuid, Tile>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));
static POSITION: LazyLock<RwLock<Vec2<f32>>> = LazyLock::new(|| RwLock::new(Vec2::zero()));
static ORIENTATION: LazyLock<RwLock<Vec2<f32>>> =
    LazyLock::new(|| RwLock::new(Vec2::new(1.0, 0.0))); // Default facing east

// Gets or add the texture of the given name and returns its id
fn get_texture(texture_name: &str) -> Option<Uuid> {
    let mut tiles = TILES.write().unwrap();

    if let Some(id) = tiles
        .iter()
        .find(|(_, tile)| tile.name == texture_name)
        .map(|(uuid, _)| *uuid)
    {
        Some(id)
    } else if let Some(tex) = load_texture(texture_name) {
        let tile = Tile::from_texture(texture_name, tex);
        let id = tile.id;

        tiles.insert(id, tile);

        Some(id)
    } else {
        None
    }
}

fn set_default_wall_texture(texture_name: String) -> PyResult<()> {
    if let Some(id) = get_texture(&texture_name) {
        *DEFAULT_WALL_TEXTURE.write().unwrap() = Some(id);
    }
    Ok(())
}

fn set_default_floor_texture(texture_name: String) -> PyResult<()> {
    if let Some(id) = get_texture(&texture_name) {
        *DEFAULT_FLOOR_TEXTURE.write().unwrap() = Some(id);
    }
    Ok(())
}

fn set_default_ceiling_texture(texture_name: String) -> PyResult<()> {
    if let Some(id) = get_texture(&texture_name) {
        *DEFAULT_CEILING_TEXTURE.write().unwrap() = Some(id);
    }
    Ok(())
}

fn move_forward(length: f32) -> PyResult<()> {
    let mut position = POSITION.write().unwrap();
    let orientation = ORIENTATION.read().unwrap();

    // Update the position based on the current orientation
    *position += *orientation * length;

    Ok(())
}

fn rotate(angle: f32) -> PyResult<()> {
    let mut orientation = ORIENTATION.write().unwrap();

    // Calculate the new orientation by rotating the vector
    let radians = angle.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();

    let new_orientation = Vec2::new(
        orientation.x * cos - orientation.y * sin,
        orientation.x * sin + orientation.y * cos,
    );

    fn snap_orientation(orientation: Vec2<f32>) -> Vec2<f32> {
        const EPSILON: f32 = 1e-5;

        let x = if orientation.x.abs() < EPSILON {
            0.0
        } else {
            orientation.x
        };
        let y = if orientation.y.abs() < EPSILON {
            0.0
        } else {
            orientation.y
        };

        Vec2::new(x, y).normalized()
    }

    *orientation = snap_orientation(new_orientation);

    Ok(())
}

fn turn_left() -> PyResult<()> {
    rotate(-90.0)
}

fn turn_right() -> PyResult<()> {
    rotate(90.0)
}

fn wall(length: f32) -> PyResult<()> {
    let mut map = MAP.write().unwrap();
    let mut position = POSITION.write().unwrap();
    let orientation = ORIENTATION.read().unwrap();

    // Calculate the "to" position based on the current orientation
    let to = *position + *orientation * length;

    // Add vertices to the map
    let from_index = map.add_vertex_at(position.x, position.y);
    let to_index = map.add_vertex_at(to.x, to.y);

    // Create the linedef
    let (linedef_id, sector_id) = map.create_linedef(from_index, to_index);

    if let Some(linedef) = map.find_linedef_mut(linedef_id) {
        linedef.texture = *DEFAULT_WALL_TEXTURE.read().unwrap();
        linedef.wall_height = 2.0;
    }

    if let Some(sector_id) = sector_id {
        if let Some(sector) = map.find_sector_mut(sector_id) {
            sector.floor_texture = *DEFAULT_FLOOR_TEXTURE.read().unwrap();
            sector.ceiling_texture = *DEFAULT_CEILING_TEXTURE.read().unwrap();
        }
    }

    // Update the current position
    *position = to;

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
    pub fn run(&mut self, source: String) -> Result<MapMeta, Vec<String>> {
        self.error = None;
        *MAP.write().unwrap() = Map::default();
        *TILES.write().unwrap() = FxHashMap::default();
        *POSITION.write().unwrap() = Vec2::zero();
        *ORIENTATION.write().unwrap() = Vec2::new(1.0, 0.0);
        *DEFAULT_WALL_TEXTURE.write().unwrap() = None;
        *DEFAULT_CEILING_TEXTURE.write().unwrap() = None;
        *DEFAULT_FLOOR_TEXTURE.write().unwrap() = None;

        let interpreter = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        interpreter.enter(|vm| {
            let scope = vm.new_scope_with_builtins();

            let _ = scope.globals.set_item(
                "set_default_wall_texture",
                vm.new_function("set_default_wall_texture", set_default_wall_texture)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_default_floor_texture",
                vm.new_function("set_default_floor_texture", set_default_floor_texture)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_default_ceiling_texture",
                vm.new_function("set_default_ceiling_texture", set_default_ceiling_texture)
                    .into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("wall", vm.new_function("wall", wall).into(), vm);

            let _ = scope.globals.set_item(
                "move_forward",
                vm.new_function("turn_left", move_forward).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "turn_left",
                vm.new_function("turn_left", turn_left).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "turn_right",
                vm.new_function("turn_right", turn_right).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("rotate", vm.new_function("rotate", rotate).into(), vm);

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

            let meta = MapMeta::new(MAP.read().unwrap().clone(), TILES.read().unwrap().clone());
            Ok(meta)
        })
    }
}
