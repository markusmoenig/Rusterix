use crate::prelude::*;
use rect_packer::{Config, Packer};
use std::path::Path;
use theframework::prelude::*;

#[derive(Clone)]
pub struct Assets {
    pub map_sources: FxHashMap<String, String>,
    pub maps: FxHashMap<String, Map>,

    pub entities: FxHashMap<String, (String, String)>,
    pub items: FxHashMap<String, (String, String)>,

    pub tiles: FxHashMap<Uuid, Tile>,
    pub materials: FxHashMap<Uuid, Tile>,
    pub textures: FxHashMap<String, Texture>,

    pub screens: FxHashMap<String, Map>,

    pub config: String,
    pub atlas: Texture,

    pub fonts: FxHashMap<String, fontdue::Font>,
    pub palette: ThePalette,
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl Assets {
    pub fn new() -> Self {
        Self {
            map_sources: FxHashMap::default(),
            maps: FxHashMap::default(),
            entities: FxHashMap::default(),
            items: FxHashMap::default(),
            tiles: FxHashMap::default(),
            textures: FxHashMap::default(),
            materials: FxHashMap::default(),
            screens: FxHashMap::default(),
            config: String::new(),
            atlas: Texture::default(),
            fonts: FxHashMap::default(),
            palette: ThePalette::default(),
        }
    }

    /// Set the tiles and atlas from a list of RGBA tiles.
    pub fn set_rgba_tiles(&mut self, textures: FxHashMap<Uuid, TheRGBATile>) {
        let atlas_size = 1024;

        let mut packer = Packer::new(Config {
            width: atlas_size,
            height: atlas_size,
            border_padding: 0,
            rectangle_padding: 1,
        });

        let mut tiles: FxHashMap<Uuid, Tile> = FxHashMap::default();
        let mut elements: FxHashMap<Uuid, Vec<vek::Vec4<i32>>> = FxHashMap::default();

        for (id, t) in textures.iter() {
            let mut array: Vec<vek::Vec4<i32>> = vec![];
            let mut texture_array: Vec<Texture> = vec![];
            for b in &t.buffer {
                if let Some(rect) = packer.pack(b.dim().width, b.dim().height, false) {
                    array.push(vek::Vec4::new(rect.x, rect.y, rect.width, rect.height));
                }

                let texture = Texture::new(
                    b.pixels().to_vec(),
                    b.dim().width as usize,
                    b.dim().height as usize,
                );
                texture_array.push(texture);
            }
            let tile = Tile {
                id: t.id,
                uvs: array.clone(),
                textures: texture_array.clone(),
                blocking: t.blocking,
                scale: t.scale,
                render_mode: t.render_mode,
            };
            elements.insert(*id, array);
            tiles.insert(*id, tile);
        }

        // Create atlas
        let mut atlas = vec![0; atlas_size as usize * atlas_size as usize * 4];

        // Copy textures into atlas
        for (id, tile) in textures.iter() {
            if let Some(rects) = elements.get(id) {
                for (buffer, rect) in tile.buffer.iter().zip(rects) {
                    let width = buffer.dim().width as usize;
                    let height = buffer.dim().height as usize;
                    let rect_x = rect.x as usize;
                    let rect_y = rect.y as usize;

                    for y in 0..height {
                        for x in 0..width {
                            let src_index = (y * width + x) * 4;
                            let dest_index =
                                ((rect_y + y) * atlas_size as usize + (rect_x + x)) * 4;

                            atlas[dest_index..dest_index + 4]
                                .copy_from_slice(&buffer.pixels()[src_index..src_index + 4]);
                        }
                    }
                }
            }
        }

        self.atlas = Texture::new(atlas, atlas_size as usize, atlas_size as usize);
        self.tiles = tiles;
    }

    /// Compile the materials.
    pub fn set_materials(&mut self, mut materials: FxHashMap<Uuid, Map>) {
        let mut tiles = FxHashMap::default();
        for map in materials.values_mut() {
            if let Some(Value::Texture(texture)) = map.properties.get("material") {
                tiles.insert(map.id, Tile::from_texture(texture.clone()));
            }
        }
        self.materials = tiles;
    }

    /// Returns an FxHashSet of Uuid representing the blocking tiles and materials.
    pub fn blocking_tiles(&self) -> FxHashSet<Uuid> {
        let mut blocking_tiles = FxHashSet::default();
        for tile in self.tiles.values() {
            if tile.blocking {
                blocking_tiles.insert(tile.id);
            }
        }
        for mat in self.materials.values() {
            if mat.blocking {
                blocking_tiles.insert(mat.id);
            }
        }
        blocking_tiles
    }

    /// Collects the assets from a directory.
    pub fn collect_from_directory(&mut self, dir_path: String) {
        let path = Path::new(&dir_path);

        if !path.is_dir() {
            eprintln!("Error: '{}' is not a directory.", path.display());
            return;
        }

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
                    match extension {
                        // Texture
                        "png" | "PNG" => {
                            if let Some(tex) = Texture::from_image_safe(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.textures.insert(base_name.to_string(), tex);
                                }
                            }
                        }
                        // Entity
                        "rxe" => {
                            if let Ok(source) = std::fs::read_to_string(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.entities
                                        .insert(base_name.to_string(), (source, String::new()));
                                }
                            }
                        }
                        // Map
                        "rxm" => {
                            if let Ok(source) = std::fs::read_to_string(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.map_sources.insert(base_name.to_string(), source);
                                }
                            }
                        }
                        _ => {
                            // println!("Unsupported file extension: {:?}", extension)
                        }
                    }
                }
            }
        }
    }

    /// Compile all source maps
    pub fn compile_source_maps(&mut self) {
        let keys = self.map_sources.keys().cloned().collect::<Vec<String>>();
        for name in keys {
            let _ = self.compile_source_map(name);
        }
    }

    /// Compile the given source map
    pub fn compile_source_map(&mut self, name: String) -> Result<(), Vec<String>> {
        if let Some(source) = self.map_sources.get(&name) {
            let mut mapscript = MapScript::default();
            match mapscript.compile(source, &self.textures, None, None, None) {
                Ok(meta) => {
                    self.maps.insert(name, meta.map);
                    for (id, tile) in meta.tiles {
                        self.tiles.insert(id, tile);
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    /// Get a map by name.
    pub fn get_map(&self, name: &str) -> Option<&Map> {
        self.maps.get(name)
    }

    /// Add an entity.
    pub fn add_entity(&mut self, name: String, code: String, data: String) {
        self.entities.insert(name, (code, data));
    }
}
