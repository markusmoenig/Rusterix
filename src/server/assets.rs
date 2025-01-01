use std::path::Path;
use theframework::prelude::*;

use crate::prelude::*;

pub struct Assets {
    pub map_sources: FxHashMap<String, String>,
    pub maps: FxHashMap<String, Map>,
    pub entities: FxHashMap<String, String>,
    pub tiles: FxHashMap<Uuid, Tile>,
    pub textures: FxHashMap<String, Texture>,
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
            tiles: FxHashMap::default(),
            textures: FxHashMap::default(),
        }
    }

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
                        // "png" => self.handle_png(file_path),
                        // "json" => self.handle_json(file_path),
                        "py" => {
                            if let Ok(source) = std::fs::read_to_string(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.entities.insert(base_name.to_string(), source);
                                }
                            }
                        }
                        "png" | "PNG" => {
                            if let Some(tex) = Texture::from_image_safe(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.textures.insert(base_name.to_string(), tex);
                                }
                            }
                        }
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

    pub fn get_map(&self, name: &str) -> Option<&Map> {
        self.maps.get(name)
    }

    /// Add an entity.
    pub fn add_entity(&mut self, name: String, code: String) {
        self.entities.insert(name, code);
    }
}
