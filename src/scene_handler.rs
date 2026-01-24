use std::str::FromStr;

use crate::{
    Assets, BillboardMetadata, D3Camera, Item, Map, PixelSource, RenderSettings, Texture, Tile,
    Value,
};
use indexmap::IndexMap;
use rust_embed::EmbeddedFile;
use rustc_hash::FxHashMap;
use scenevm::{Atom, Chunk, DynamicObject, GeoId, Light, SceneVM};
use theframework::prelude::*;

pub struct SceneHandler {
    pub vm: SceneVM,

    pub overlay_2d_id: Uuid,
    pub overlay_2d: Chunk,

    pub overlay_3d_id: Uuid,
    pub overlay_3d: Chunk,

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

    pub settings: RenderSettings,

    // Billboards for dynamic doors/gates (indexed by GeoId for fast lookup)
    pub billboards: FxHashMap<GeoId, BillboardMetadata>,
}

impl Default for SceneHandler {
    fn default() -> Self {
        SceneHandler::empty()
    }
}

impl SceneHandler {
    pub fn find_item_any<'m>(map: &'m Map, id: u32) -> Option<&'m Item> {
        if let Some(item) = map.items.iter().find(|i| i.id == id) {
            return Some(item);
        }
        for profile_map in map.profiles.values() {
            if let Some(item) = profile_map.items.iter().find(|i| i.id == id) {
                return Some(item);
            }
        }
        None
    }

    pub fn find_item_by_profile_attrs<'m>(
        map: &'m Map,
        host_sector: Option<u32>,
        profile_sector: Option<u32>,
    ) -> Option<&'m Item> {
        let Some(host) = host_sector else { return None };
        let Some(profile) = profile_sector else {
            return None;
        };

        map.items.iter().find(|item| {
            let host_matches = match item.attributes.get("profile_host_sector_id") {
                Some(Value::UInt(v)) => *v == host,
                Some(Value::Int(v)) if *v >= 0 => *v as u32 == host,
                Some(Value::Int64(v)) if *v >= 0 => *v as u32 == host,
                _ => false,
            };
            let profile_matches = match item.attributes.get("profile_sector_id") {
                Some(Value::UInt(v)) => *v == profile,
                Some(Value::Int(v)) if *v >= 0 => *v as u32 == profile,
                Some(Value::Int64(v)) if *v >= 0 => *v as u32 == profile,
                _ => false,
            };
            host_matches && profile_matches
        })
    }

    pub fn empty() -> Self {
        let vm = SceneVM::default();
        // vm.set_layer_activity_logging(true);

        Self {
            vm,

            overlay_2d_id: Uuid::new_v4(),
            overlay_2d: Chunk::default(),

            overlay_3d_id: Uuid::new_v4(),
            overlay_3d: Chunk::default(),

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

            settings: RenderSettings::default(),

            billboards: FxHashMap::default(),
        }
    }

    pub fn build_atlas(&mut self, tiles: &IndexMap<Uuid, Tile>, editor: bool) {
        for (id, tile) in tiles {
            let mut b = vec![];
            for t in &tile.textures {
                b.push(t.data.to_vec());
            }
            self.vm.execute(Atom::AddTile {
                id: *id,
                width: tile.textures[0].width as u32,
                height: tile.textures[0].height as u32,
                frames: tile.to_buffer_array(),
                material_frames: Some(tile.to_material_array()),
            });
        }

        if editor {
            fn decode_png(file: EmbeddedFile) -> Option<(Vec<u8>, u32, u32)> {
                // Use the `image` crate to decode, auto-detecting the format from bytes.
                match image::load_from_memory(&file.data) {
                    Ok(dynamic) => {
                        let rgba = dynamic.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        Some((rgba.into_raw(), w, h))
                    }
                    Err(_) => None,
                }
            }

            if let Some(bytes) = crate::Embedded::get("icons/character_off.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.character_off,
                        width,
                        height,
                        frames: vec![bytes],
                        material_frames: None,
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
                        material_frames: None,
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
                        material_frames: None,
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
                        material_frames: None,
                    });
                }
            }
            let checker = Texture::checkerboard(100, 50);
            self.vm.execute(Atom::AddTile {
                id: Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                    .ok()
                    .unwrap(),
                width: 100,
                height: 100,
                frames: vec![checker.data],
                material_frames: None,
            });
            // self.vm.execute(Atom::AddSolid {
            //     id: Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
            //         .ok()
            //         .unwrap(),
            //     color: [128, 128, 18, 255],
            // });
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
    }

    pub fn clear_overlay(&mut self) {
        if self.vm.vm_layer_count() == 1 {
            // 2D Overlay layer
            let idx = self.vm.add_vm_layer();
            self.vm.set_active_vm(idx);

            self.vm.execute(scenevm::Atom::SetBackground(Vec4::zero()));
            if let Some(bytes) = crate::Embedded::get("shader/2d_overlay_shader.wgsl") {
                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                    self.vm.execute(Atom::SetSource2D(source.into()));
                }
            }
            self.vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));

            // 3D Overlay layer
            let idx = self.vm.add_vm_layer();
            self.vm.set_active_vm(idx);

            self.vm.execute(scenevm::Atom::SetBackground(Vec4::zero()));
            if let Some(bytes) = crate::Embedded::get("shader/3d_overlay_shader.wgsl") {
                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                    self.vm.execute(Atom::SetSource3D(source.into()));
                }
            }
            self.vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute3D));
        }
        self.vm.set_active_vm(0);

        self.overlay_2d = Chunk::default();
        self.overlay_2d.priority = 0;

        self.overlay_3d = Chunk::default();
        self.overlay_3d.priority = 0;
    }

    pub fn set_overlay(&mut self) {
        self.vm.set_active_vm(1);
        self.vm.execute(Atom::AddChunk {
            id: self.overlay_2d_id,
            chunk: self.overlay_2d.clone(),
        });
        self.vm.set_active_vm(2);
        self.vm.execute(Atom::AddChunk {
            id: self.overlay_3d_id,
            chunk: self.overlay_3d.clone(),
        });
        self.vm.set_active_vm(0);
    }

    pub fn add_overlay_2d_line(
        &mut self,
        id: GeoId,
        start: Vec2<f32>,
        end: Vec2<f32>,
        color: Uuid,
        layer: i32,
    ) {
        self.overlay_2d.add_line_strip_2d_px(
            id,
            color,
            vec![start.into_array(), end.into_array()],
            1.5,
            layer,
        );
    }

    /// Build dynamic elements of the 2D Map: Entities, Items, Lights ...
    pub fn build_dynamics_2d(&mut self, map: &Map, assets: &Assets) {
        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);

        for item in &map.items {
            let item_pos = Vec2::new(item.position.x, item.position.z);
            let pos = Vec2::new(item_pos.x, item_pos.y);

            if let Some(Value::Light(light)) = item.attributes.get("light") {
                self.vm.execute(Atom::AddLight {
                    id: GeoId::ItemLight(item.id),
                    light: Light::new_pointlight(item.position)
                        .with_color(Vec3::from(light.get_color()))
                        .with_intensity(light.get_intensity())
                        .with_emitting(light.active)
                        .with_start_distance(light.get_start_distance())
                        .with_end_distance(light.get_end_distance())
                        .with_flicker(light.get_flicker()),
                });
            }

            if let Some(Value::Source(source)) = item.attributes.get("source") {
                if item.attributes.get_bool_default("visible", false) {
                    if let Some(tile) = source.tile_from_tile_list(assets) {
                        let dynamic = DynamicObject::billboard_tile_2d(
                            GeoId::Item(item.id),
                            tile.id,
                            pos,
                            1.0,
                            1.0,
                        );
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    }
                }
            }
        }

        for entity in &map.entities {
            let entity_pos = Vec2::new(entity.position.x, entity.position.z);
            let pos = Vec2::new(entity_pos.x, entity_pos.y);

            // Find light on entity
            if let Some(Value::Light(light)) = entity.attributes.get("light") {
                if light.active {
                    let mut light = light.clone();
                    light.set_position(entity.position);
                }
            }

            // Find light on entity items
            for (_, item) in entity.iter_inventory() {
                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    if light.active {
                        self.vm.execute(Atom::AddLight {
                            id: GeoId::ItemLight(item.id),
                            light: Light::new_pointlight(entity.position)
                                .with_color(Vec3::from(light.get_color()))
                                .with_intensity(light.get_intensity())
                                .with_emitting(light.active)
                                .with_start_distance(light.get_start_distance())
                                .with_end_distance(light.get_end_distance())
                                .with_flicker(light.get_flicker()),
                        });
                    }
                }
            }

            if let Some(Value::Source(source)) = entity.attributes.get("source") {
                if entity.attributes.get_bool_default("visible", false) {
                    if let Some(tile) = source.tile_from_tile_list(assets) {
                        let dynamic = DynamicObject::billboard_tile_2d(
                            GeoId::Character(entity.id),
                            tile.id,
                            pos,
                            1.0,
                            1.0,
                        );
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    }
                }
            }
        }
    }

    pub fn build_dynamics_3d(&mut self, map: &Map, camera: &dyn D3Camera, assets: &Assets) {
        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);

        let basis = camera.basis_vectors();

        // Entities
        for entity in &map.entities {
            let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

            if show_entity {
                // Find light on entity
                if let Some(Value::Light(light)) = entity.attributes.get("light") {
                    self.vm.execute(Atom::AddLight {
                        id: GeoId::ItemLight(entity.id),
                        light: Light::new_pointlight(entity.position)
                            .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                            .with_intensity(light.get_intensity())
                            .with_emitting(light.active)
                            .with_start_distance(light.get_start_distance())
                            .with_end_distance(light.get_end_distance())
                            .with_flicker(light.get_flicker()),
                    });
                }

                // Find light on entity items
                for (_, item) in entity.iter_inventory() {
                    if let Some(Value::Light(light)) = item.attributes.get("light") {
                        self.vm.execute(Atom::AddLight {
                            id: GeoId::ItemLight(item.id),
                            light: Light::new_pointlight(entity.position)
                                .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                                .with_intensity(light.get_intensity())
                                .with_emitting(light.active)
                                .with_start_distance(light.get_start_distance())
                                .with_end_distance(light.get_end_distance())
                                .with_flicker(light.get_flicker()),
                        });
                    }
                }

                if let Some(Value::Source(source)) = entity.attributes.get("source") {
                    if entity.attributes.get_bool_default("visible", false) {
                        let size = 2.0;
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            let center3 =
                                Vec3::new(entity.position.x, size * 0.5, entity.position.z);

                            let dynamic = DynamicObject::billboard_tile(
                                GeoId::Item(entity.id),
                                tile.id,
                                center3,
                                basis.1,
                                basis.2,
                                size,
                                size,
                            );
                            self.vm.execute(Atom::AddDynamic { object: dynamic });
                        }
                    }
                }
            }

            // Items
            for item in &map.items {
                let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

                if show_entity {
                    if let Some(Value::Light(light)) = item.attributes.get("light") {
                        self.vm.execute(Atom::AddLight {
                            id: GeoId::ItemLight(item.id),
                            light: Light::new_pointlight(item.position)
                                .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                                .with_intensity(light.get_intensity())
                                .with_emitting(light.active)
                                .with_start_distance(light.get_start_distance())
                                .with_end_distance(light.get_end_distance())
                                .with_flicker(light.get_flicker()),
                        });
                    }

                    if let Some(Value::Source(source)) = item.attributes.get("source") {
                        if item.attributes.get_bool_default("visible", false) {
                            let size = 1.0;
                            if let Some(tile) = source.tile_from_tile_list(assets) {
                                let center3 =
                                    Vec3::new(item.position.x, size * 0.5, item.position.z);

                                let dynamic = DynamicObject::billboard_tile(
                                    GeoId::Item(item.id),
                                    tile.id,
                                    center3,
                                    basis.1,
                                    basis.2,
                                    size,
                                    size,
                                );
                                self.vm.execute(Atom::AddDynamic { object: dynamic });
                            }
                        }
                    }
                }
            }
        }

        // Vertices with billboards
        for vertex in &map.vertices {
            if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                vertex.properties.get("source")
            {
                let size = vertex.properties.get_float_default("source_size", 1.0);
                let center3 = Vec3::new(vertex.x, vertex.z + size * 0.5, vertex.y);

                let dynamic = DynamicObject::billboard_tile(
                    GeoId::Vertex(vertex.id),
                    *tile_id,
                    center3,
                    basis.1,
                    basis.2,
                    size,
                    size,
                );
                self.vm.execute(Atom::AddDynamic { object: dynamic });
            }
        }

        // Billboards (doors/gates)
        for (geo_id, billboard) in &self.billboards {
            // Doors/gates use GeoId::Hole(host_sector, profile_sector)
            let resolved_item = match geo_id {
                GeoId::Hole(host, profile) => {
                    Self::find_item_by_profile_attrs(map, Some(*host), Some(*profile))
                }
                _ => None,
            };

            let is_visible = resolved_item
                .map(|item| item.attributes.get_bool_default("visible", true))
                .unwrap_or(true);

            if is_visible {
                // Calculate animation offset based on animation type and state
                // For now, render at static position (you can add animation interpolation later)
                let animated_center = billboard.center;

                let dynamic = DynamicObject::billboard_tile(
                    *geo_id,
                    billboard.tile_id,
                    animated_center,
                    billboard.up,
                    billboard.right,
                    billboard.size,
                    billboard.size,
                )
                .with_repeat_mode(billboard.repeat_mode);
                self.vm.execute(Atom::AddDynamic { object: dynamic });
            }
        }
    }
}
