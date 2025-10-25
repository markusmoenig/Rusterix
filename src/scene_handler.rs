use scenevm::{Atom, Chunk, GeoId, SceneVM};
use theframework::prelude::*;

pub struct SceneHandler {
    pub vm: SceneVM,

    pub overlay_2d_id: Uuid,
    pub overlay_2d: Chunk,

    pub white: Uuid,
    pub selected: Uuid,
    pub gray: Uuid,
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

            white: Uuid::new_v4(),
            selected: Uuid::new_v4(),
            gray: Uuid::new_v4(),
        }
    }

    pub fn build_atlas(&mut self, tiles: &FxHashMap<Uuid, TheRGBATile>) {
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

        self.vm.execute(Atom::AddSolid {
            id: self.white,
            color: [255, 255, 255, 255],
        });
        self.vm.execute(Atom::AddSolid {
            id: self.selected,
            color: [187, 122, 208, 255],
        });

        self.vm.execute(Atom::BuildAtlas);
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
            0.18,
            layer,
        );
    }
}
