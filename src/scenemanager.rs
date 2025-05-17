use crate::{Assets, BBox, Chunk, ChunkBuilder, Map};
use crossbeam::channel::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
pub enum SceneManagerCmd {
    SetTextures(FxHashMap<Uuid, TheRGBATile>),
    SetMaterials(FxHashMap<Uuid, Map>),
    SetPalette(ThePalette),
    SetMap(Map),
    Quit,
}

#[derive(Debug)]
pub enum SceneManagerResult {
    Startup,
    Quit,
}

#[derive()]
pub struct SceneManager {
    pub tx: Option<Sender<SceneManagerCmd>>,
    pub rx: Option<Receiver<SceneManagerResult>>,

    renderer_thread: Option<JoinHandle<()>>,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            tx: None,
            rx: None,

            renderer_thread: None,
        }
    }

    /// Check for a result
    pub fn receive(&self) -> Option<SceneManagerResult> {
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                return Some(result);
            }
        }

        None
    }

    /// Send a cmd.
    pub fn send(&self, cmd: SceneManagerCmd) {
        if let Some(tx) = &self.tx {
            tx.send(cmd).unwrap();
        }
    }

    pub fn set_textures(&self, textures: FxHashMap<Uuid, TheRGBATile>) {
        self.send(SceneManagerCmd::SetTextures(textures));
    }

    pub fn set_materials(&self, materials: FxHashMap<Uuid, Map>) {
        self.send(SceneManagerCmd::SetMaterials(materials));
    }

    pub fn set_palette(&self, palette: ThePalette) {
        self.send(SceneManagerCmd::SetPalette(palette));
    }

    pub fn set_map(&self, map: Map) {
        self.send(SceneManagerCmd::SetMap(map));
    }

    pub fn startup(&mut self) {
        let (tx, rx) = channel::unbounded::<SceneManagerCmd>();
        self.tx = Some(tx);
        let (result_tx, result_rx) = channel::unbounded::<SceneManagerResult>();
        self.rx = Some(result_rx);

        result_tx.send(SceneManagerResult::Startup).unwrap();

        let mut assets = Assets::default();
        let mut map = Map::default();
        let chunk_size = 16;
        let mut dirty: FxHashSet<(i32, i32)> = FxHashSet::default();

        let mut chunk_builder_d2: Option<Box<dyn ChunkBuilder>> = None;

        let mut exit_loop = false;
        self.renderer_thread = Some(thread::spawn(move || {
            loop {
                if exit_loop {
                    break;
                }
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        SceneManagerCmd::SetTextures(textures) => {
                            println!("SceneManagerCmd::SetTextures({})", textures.len());
                            assets.set_rgba_tiles(textures);
                        }
                        SceneManagerCmd::SetMaterials(materials) => {
                            println!("SceneManagerCmd::SetMaterials({})", materials.len());
                            assets.set_materials(materials);
                        }
                        SceneManagerCmd::SetPalette(palette) => {
                            println!("SceneManagerCmd::SetPalette()");
                            assets.palette = palette;
                        }
                        SceneManagerCmd::SetMap(new_map) => {
                            map = new_map;
                            let bbox = map.bbox();
                            println!(
                                "SceneManagerCmd::SetMap(Min: {}, Max: {})",
                                bbox.min, bbox.max
                            );

                            dirty = Self::generate_chunk_coords(&bbox, chunk_size);
                        }
                        SceneManagerCmd::Quit => {
                            exit_loop = true;
                        }
                    }
                }

                // Process one chunk
                if let Some(&coord) = dirty.iter().next() {
                    dirty.remove(&coord);

                    println!("Processing chunk at {:?}", coord);

                    let mut chunk = Chunk::new(coord);

                    if let Some(cb_d2) = &mut chunk_builder_d2 {
                        cb_d2.build(&map, &assets, &mut chunk);
                    }
                }
            }
        }));
    }

    /// Returns all chunks which cover the given bounding box.
    fn generate_chunk_coords(bbox: &BBox, chunk_size: i32) -> FxHashSet<(i32, i32)> {
        let min_x = (bbox.min.x / chunk_size as f32).floor() as i32;
        let min_y = (bbox.min.y / chunk_size as f32).floor() as i32;
        let max_x = (bbox.max.x / chunk_size as f32).ceil() as i32;
        let max_y = (bbox.max.y / chunk_size as f32).ceil() as i32;

        let mut coords = FxHashSet::default();
        for y in min_y..max_y {
            for x in min_x..max_x {
                coords.insert((x, y));
            }
        }
        coords
    }
}
