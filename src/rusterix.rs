use crate::prelude::*;
use std::path::Path;

/// Rusterix can server as a server or client or both for solo games.
pub struct Rusterix {
    pub server: Server,
}

impl Default for Rusterix {
    fn default() -> Self {
        Self::new()
    }
}

impl Rusterix {
    pub fn new() -> Self {
        Self {
            server: Server::default(),
        }
    }

    pub fn from_directory(&mut self, dir_path: String) {
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
                        "rxm" => {
                            //println!("found .rxm {:?})", file_path)
                            let mut mapscript = MapScript::with_path(dir_path.clone());
                            if let Some(p) = file_path.to_str() {
                                mapscript.load_map(p);
                                match mapscript.transform(None, None, None) {
                                    Ok(meta) => {
                                        println!("ok");
                                        self.add_region(meta);
                                    }
                                    Err(err) => {
                                        println!("error");
                                    }
                                }
                            }
                        }
                        _ => println!("Unsupported file extension: {:?}", extension),
                    }
                }
            }
        }
    }

    pub fn add_region(&mut self, meta: MapMeta) {
        let mut region = Region::default();
    }
}
