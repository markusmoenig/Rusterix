use crate::ShapeStack;
use crate::prelude::*;

use indexmap::IndexMap;
use vek::Vec2;

/// Builds tiles for entities and items
pub fn tile_builder(map: &mut Map, assets: &mut Assets) {
    let size = 32;

    for entity in map.entities.iter() {
        // Check if we have a sequence to build / check
        if let Some(PixelSource::Sequence(name)) = entity.attributes.get_source("source") {
            if let Some(entity_tiles) = assets.entity_tiles.get(&entity.id) {
                if !entity_tiles.contains_key(name) {
                    // No sequence of this name for the entity, build the sequence
                    println!(
                        "No sequences ({}) for {}",
                        name,
                        entity.attributes.get_str_default("name", "unknown".into())
                    );
                }
            } else {
                // No sequences for this character at all, build the sequence
                println!(
                    "No sequences at all ({}) for {}",
                    name,
                    entity.attributes.get_str_default("name", "unknown".into())
                );

                if let Some(Value::Str(class_name)) = entity.attributes.get("class_name") {
                    if let Some(character_map) = assets.character_maps.get(class_name) {
                        let mut texture = Texture::alloc(size as usize, size as usize);
                        let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
                        stack.render_geometry(&mut texture, character_map, assets, false);

                        let tile = Tile::from_texture(texture);
                        let mut states: IndexMap<String, Tile> = IndexMap::default();
                        states.insert(name.clone(), tile);

                        assets.entity_tiles.insert(entity.id, states);
                    }
                }
            }
        }
    }
}
