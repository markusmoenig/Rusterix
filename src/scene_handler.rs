use rust_embed::EmbeddedFile;
use scenevm::{Atom, Chunk, GeoId, Material, SceneVM};
use theframework::prelude::*;

pub struct SceneHandler {
    pub vm: SceneVM,

    pub overlay_2d_id: Uuid,
    pub overlay_2d: Chunk,

    pub character_off: Uuid,
    pub character_on: Uuid,
    pub item_off: Uuid,
    pub item_on: Uuid,

    pub flat_material: Uuid,

    pub white: Uuid,
    pub selected: Uuid,
    pub gray: Uuid,
    pub outline: Uuid,
    pub yellow: Uuid,
}

impl Default for SceneHandler {
    fn default() -> Self {
        SceneHandler::empty()
    }
}

impl SceneHandler {
    pub fn empty() -> Self {
        Self {
            vm: SceneVM::default(),

            overlay_2d_id: Uuid::new_v4(),
            overlay_2d: Chunk::default(),

            character_off: Uuid::new_v4(),
            character_on: Uuid::new_v4(),
            item_off: Uuid::new_v4(),
            item_on: Uuid::new_v4(),

            flat_material: Uuid::new_v4(),

            white: Uuid::new_v4(),
            selected: Uuid::new_v4(),
            gray: Uuid::new_v4(),
            outline: Uuid::new_v4(),
            yellow: Uuid::new_v4(),
        }
    }

    pub fn build_atlas(&mut self, tiles: &FxHashMap<Uuid, TheRGBATile>, editor: bool) {
        for (id, tile) in tiles {
            let mut b = vec![];
            for t in &tile.buffer {
                b.push(t.pixels().to_vec());
            }
            self.vm.execute(Atom::AddTile {
                id: *id,
                width: tile.buffer[0].dim().width as u32,
                height: tile.buffer[0].dim().height as u32,
                frames: b,
            });
        }

        if editor {
            fn decode_png(file: EmbeddedFile) -> Option<(Vec<u8>, u32, u32)> {
                let data = std::io::Cursor::new(file.data);

                let decoder = png::Decoder::new(data);
                if let Ok(mut reader) = decoder.read_info() {
                    let mut buf = vec![0; reader.output_buffer_size()];
                    let info = reader.next_frame(&mut buf).unwrap();
                    let bytes = &buf[..info.buffer_size()];

                    // Ensure the image data has 4 channels (RGBA)
                    let rgba_bytes = if info.color_type.samples() == 3 {
                        // Image is RGB, expand to RGBA
                        let mut expanded_buf =
                            Vec::with_capacity(info.width as usize * info.height as usize * 4);
                        for chunk in bytes.chunks(3) {
                            expanded_buf.push(chunk[0]); // R
                            expanded_buf.push(chunk[1]); // G
                            expanded_buf.push(chunk[2]); // B
                            expanded_buf.push(255); // A (opaque)
                        }
                        expanded_buf
                    } else {
                        // Image is already RGBA
                        bytes.to_vec()
                    };

                    return Some((rgba_bytes, info.width, info.height));
                }
                None
            }

            if let Some(bytes) = crate::Embedded::get("icons/character_off.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.character_off,
                        width,
                        height,
                        frames: vec![bytes],
                    });
                }
            }
            if let Some(bytes) = crate::Embedded::get("icons/character_on.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.character_on,
                        width,
                        height,
                        frames: vec![bytes],
                    });
                }
            }
            if let Some(bytes) = crate::Embedded::get("icons/treasure_off.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.item_off,
                        width,
                        height,
                        frames: vec![bytes],
                    });
                }
            }
            if let Some(bytes) = crate::Embedded::get("icons/treasure_on.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.item_on,
                        width,
                        height,
                        frames: vec![bytes],
                    });
                }
            }
            self.vm.execute(Atom::AddSolid {
                id: self.white,
                color: [255, 255, 255, 255],
            });
            self.vm.execute(Atom::AddSolid {
                id: self.selected,
                color: [187, 122, 208, 255],
            });
            self.vm.execute(Atom::AddSolid {
                id: self.outline,
                color: [122, 208, 187, 255],
            });
            self.vm.execute(Atom::AddSolid {
                id: self.yellow,
                color: vek::Rgba::yellow().into_array(),
            });
        }

        self.vm.execute(Atom::BuildAtlas);

        self.vm.execute(Atom::AddMaterial {
            id: self.flat_material,
            material: Material::flat(),
        });
    }

    pub fn clear_overlay_2d(&mut self) {
        self.overlay_2d = Chunk::default();
        self.overlay_2d.priority = 1;
    }

    pub fn set_overlay_2d(&mut self) {
        self.vm.execute(Atom::AddChunk {
            id: self.overlay_2d_id,
            chunk: self.overlay_2d.clone(),
        });
    }

    pub fn add_overlay_2d_line(
        &mut self,
        id: GeoId,
        start: Vec2<f32>,
        end: Vec2<f32>,
        color: Uuid,
        layer: i32,
    ) {
        self.overlay_2d.add_line_strip_2d(
            id,
            color,
            vec![start.into_array(), end.into_array()],
            0.1,
            layer,
            Some(self.flat_material),
        );
    }
}
