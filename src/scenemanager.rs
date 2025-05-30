use crate::{
    Assets, BBox, Batch3D, Chunk, ChunkBuilder, D2ChunkBuilder, D3ChunkBuilder, Map, TerrainChunk,
    Tile,
};
use crossbeam::channel::{self, Receiver, Sender};
// use rayon::prelude::*;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
pub enum SceneManagerCmd {
    SetTileList(Vec<Tile>, FxHashMap<Uuid, u16>),
    SetPalette(ThePalette),
    SetMap(Map),
    SetBuilder2D(Option<Box<dyn ChunkBuilder>>),
    AddDirty(Vec<(i32, i32)>),
    SetDirtyTerrainChunks(Vec<TerrainChunk>),
    SetTerrainModifierState(bool),
    Quit,
}

// #[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum SceneManagerResult {
    Startup,
    Clear,
    Chunk(Chunk, i32, i32),
    ProcessedHeights(Vec2<i32>, FxHashMap<(i32, i32), f32>),
    UpdatedBatch3D((i32, i32), Batch3D),
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

    pub fn set_tile_list(&self, tiles: Vec<Tile>, tile_indices: FxHashMap<Uuid, u16>) {
        self.send(SceneManagerCmd::SetTileList(tiles, tile_indices));
    }

    pub fn set_palette(&self, palette: ThePalette) {
        self.send(SceneManagerCmd::SetPalette(palette));
    }

    pub fn set_builder_2d(&self, builder: Option<Box<dyn ChunkBuilder>>) {
        self.send(SceneManagerCmd::SetBuilder2D(builder));
    }

    pub fn set_map(&self, map: Map) {
        self.send(SceneManagerCmd::SetMap(map));
    }

    pub fn add_dirty(&self, dirty: Vec<(i32, i32)>) {
        self.send(SceneManagerCmd::AddDirty(dirty));
    }

    pub fn set_dirty_terrain_chunks(&self, dirty: Vec<TerrainChunk>) {
        self.send(SceneManagerCmd::SetDirtyTerrainChunks(dirty));
    }

    pub fn set_terrain_modifier_state(&self, state: bool) {
        self.send(SceneManagerCmd::SetTerrainModifierState(state));
    }

    pub fn startup(&mut self) {
        let (tx, rx) = channel::unbounded::<SceneManagerCmd>();
        self.tx = Some(tx);
        let (result_tx, result_rx) = channel::unbounded::<SceneManagerResult>();
        self.rx = Some(result_rx);

        result_tx.send(SceneManagerResult::Startup).unwrap();

        let mut assets = Assets::default();
        let mut map = Map::default();
        let mut map_geo = Map::default();
        let mut terrain_modifiers = true;

        let chunk_size = 16;

        let mut dirty: FxHashSet<(i32, i32)> = FxHashSet::default();
        let mut all: FxHashSet<(i32, i32)> = FxHashSet::default();
        let mut terrain_modifiers_update: FxHashSet<(i32, i32)> = FxHashSet::default();

        let mut total_chunks = 0;

        let mut chunk_builder_d2: Option<Box<dyn ChunkBuilder>> =
            Some(Box::new(D2ChunkBuilder::new()));

        let mut chunk_builder_d3: Option<Box<dyn ChunkBuilder>> =
            Some(Box::new(D3ChunkBuilder::new()));

        let tick = crossbeam::channel::tick(Duration::from_millis(5));
        self.renderer_thread = Some(thread::spawn(move || {
            loop {
                crossbeam::select! {
                    recv(rx) -> msg => {
                        match msg {
                            Ok(cmd) => {
                                match cmd {
                                    SceneManagerCmd::SetTileList(tiles, indices) => {
                                        println!("SceneManagerCmd::SetTileList({})", tiles.len());
                                        assets.tile_list = tiles;
                                        assets.tile_indices = indices;
                                        dirty = Self::generate_chunk_coords(&map.bbox(), chunk_size);
                                        all = dirty.clone();
                                    }
                                    SceneManagerCmd::SetPalette(palette) => {
                                        println!("SceneManagerCmd::SetPalette()");
                                        assets.palette = palette;
                                        dirty = Self::generate_chunk_coords(&map.bbox(), chunk_size);
                                        all = dirty.clone();
                                    }
                                    SceneManagerCmd::SetBuilder2D(builder) => {
                                        println!("SceneManagerCmd::SetBuilder2D()");
                                        chunk_builder_d2 = builder;
                                        dirty = Self::generate_chunk_coords(&map.bbox(), chunk_size);
                                        all = dirty.clone();
                                    }
                                    SceneManagerCmd::SetMap(new_map) => {
                                        if map.id != new_map.id {
                                            result_tx.send(SceneManagerResult::Clear).ok();
                                        }
                                        map = new_map;
                                        map_geo = map.geometry_clone();
                                        let mut bbox = map.bbox();
                                        if let Some(tbbox) = map.terrain.compute_bounds() {
                                            bbox.expand_bbox(tbbox);
                                        }
                                        println!(
                                            "SceneManagerCmd::SetMap(Min: {}, Max: {})",
                                            bbox.min, bbox.max
                                        );
                                        dirty = Self::generate_chunk_coords(&bbox, chunk_size);
                                        all = dirty.clone();
                                        total_chunks = dirty.len() as i32;
                                    }
                                    SceneManagerCmd::AddDirty(dirty_chunks) => {
                                        for d in dirty_chunks {
                                            dirty.insert(d);
                                            all.insert(d);
                                        }
                                    }
                                    SceneManagerCmd::SetDirtyTerrainChunks(dirty_chunks) => {
                                        for chunk in dirty_chunks {
                                            let coord = (chunk.origin.x, chunk.origin.y);
                                            let local = map.terrain.get_chunk_coords(coord.0, coord.1);
                                            map.terrain.chunks.insert(local, chunk);
                                            dirty.insert(coord);
                                            all.insert(coord);
                                            if !terrain_modifiers {
                                                terrain_modifiers_update.insert(coord);
                                            }
                                        }
                                    }
                                    SceneManagerCmd::SetTerrainModifierState(state) => {
                                        if state && !terrain_modifiers {
                                            // Update all the chunks we created w/o modifiers
                                            for d in &terrain_modifiers_update {
                                                dirty.insert(*d);
                                                all.insert(*d);
                                            }
                                        }
                                        terrain_modifiers = state;
                                        terrain_modifiers_update.clear();
                                    }
                                    SceneManagerCmd::Quit => {
                                        result_tx.send(SceneManagerResult::Quit).ok();
                                        return;
                                    }
                                }
                            }
                            Err(_) => {
                                println!("SceneManager: channel closed");
                                return;
                            }
                        }
                    },
                    recv(tick) -> _ => {
                        if let Some(&coord) = dirty.iter().next() {
                            dirty.remove(&coord);

                            // println!("Processing chunk at {:?}", coord);

                            let mut chunk = Chunk::new(Vec2::new(coord.0, coord.1), chunk_size);

                            if let Some(cb_d2) = &mut chunk_builder_d2 {
                                cb_d2.build(&map, &assets, &mut chunk);
                            }

                            if let Some(cb_d3) = &mut chunk_builder_d3 {
                                cb_d3.build(&map, &assets, &mut chunk);
                                for chunk3d in &mut chunk.batches3d {
                                    chunk3d.compute_vertex_normals();
                                }
                            }

                            let local = map.terrain.get_chunk_coords(coord.0, coord.1);
                            if map.terrain.chunks.contains_key(&local) {
                                map.terrain.build_chunk_at(local, &assets, &map_geo, 32, &mut chunk, terrain_modifiers);
                                if let Some(ch) = map.terrain.chunks.get_mut(&local).cloned() {
                                    chunk.terrain_batch2d = Some(ch.build_mesh_d2(&map.terrain));
                                    chunk.terrain_batch3d = Some(ch.build_mesh(&map.terrain));
                                    if let Some(ph) = ch.processed_heights {
                                        result_tx.send(SceneManagerResult::ProcessedHeights(Vec2::new(coord.0, coord.1), ph)).ok();
                                    }
                                }
                            }

                            // Send the chunk
                            result_tx.send(SceneManagerResult::Chunk(chunk, dirty.len() as i32, total_chunks)).ok();

                            // Drain all queued up ticks to make sure messages are received first
                            while tick.try_recv().is_ok() {}

                            if dirty.is_empty() {
                                // When finished we need to recompute all 3D terrain meshes
                                // to correct cross boundary errors
                                for coord in &all {
                                    let local = map.terrain.get_chunk_coords(coord.0, coord.1);
                                    if map.terrain.chunks.contains_key(&local) {
                                        if let Some(ch) = map.terrain.chunks.get_mut(&local).cloned() {
                                            let batch = ch.build_mesh(&map.terrain);
                                            if !batch.vertices.is_empty() {
                                                result_tx.send(SceneManagerResult::UpdatedBatch3D(*coord, batch)).ok();
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
                coords.insert((x * chunk_size, y * chunk_size));
            }
        }
        coords
    }
}
