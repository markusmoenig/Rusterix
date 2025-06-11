use crate::ShapeStack;
use crate::prelude::*;

use indexmap::IndexMap;
use vek::Vec2;

/// Builds tiles for entities and items
pub fn tile_builder(map: &mut Map, assets: &mut Assets) {
    let size = 64;

    for entity in map.entities.iter() {
        if entity.attributes.contains("source") {
            continue;
        }

        // Check if we have a sequence to build / check
        if let Some(PixelSource::Sequence(name)) = entity.attributes.get_source("_source_seq") {
            if let Some(entity_tiles) = assets.entity_tiles.get(&entity.id) {
                if !entity_tiles.contains_key(name) {
                    // No sequence of this name for the entity, build the sequence
                    println!(
                        "No sequences ({}) for {}",
                        name,
                        entity.attributes.get_str_default("name", "unknown".into())
                    );

                    if let Some(Value::Str(class_name)) = entity.attributes.get("class_name") {
                        if let Some(character_map) = assets.character_maps.get(class_name) {
                            let tile = build_tile(character_map, assets, name, size);
                            if let Some(entity_tiles) = assets.entity_tiles.get_mut(&entity.id) {
                                entity_tiles.insert(name.clone(), tile);
                            }
                        }
                    }
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
                        let tile = build_tile(character_map, assets, name, size);
                        let mut states: IndexMap<String, Tile> = IndexMap::default();
                        states.insert(name.clone(), tile);

                        assets.entity_tiles.insert(entity.id, states);
                    }
                }
            }
        }
    }

    for item in map.items.iter() {
        if item.attributes.contains("source") {
            continue;
        }

        // Check if we have a sequence to build / check
        if let Some(PixelSource::Sequence(name)) = item.attributes.get_source("_source_seq") {
            if let Some(item_tiles) = assets.item_tiles.get(&item.id) {
                if !item_tiles.contains_key(name) {
                    // No sequence of this name for the entity, build the sequence
                    println!(
                        "No sequences ({}) for {}",
                        name,
                        item.attributes.get_str_default("name", "unknown".into())
                    );

                    if let Some(Value::Str(class_name)) = item.attributes.get("class_name") {
                        if let Some(item_map) = assets.item_maps.get(class_name) {
                            let tile = build_tile(item_map, assets, name, size);
                            if let Some(item_tiles) = assets.entity_tiles.get_mut(&item.id) {
                                item_tiles.insert(name.clone(), tile);
                            }
                        }
                    }
                }
            } else {
                // No sequences for this character at all, build the sequence
                println!(
                    "No sequences at all ({}) for {}",
                    name,
                    item.attributes.get_str_default("name", "unknown".into())
                );

                if let Some(Value::Str(class_name)) = item.attributes.get("class_name") {
                    if let Some(item_map) = assets.item_maps.get(class_name) {
                        let tile = build_tile(item_map, assets, name, size);
                        let mut states: IndexMap<String, Tile> = IndexMap::default();
                        states.insert(name.clone(), tile);

                        assets.item_tiles.insert(item.id, states);
                    }
                }
            }
        }
    }
}

fn build_tile(map: &Map, assets: &Assets, base_sequence: &str, size: i32) -> Tile {
    let mut matched_rigs: Vec<(&SoftRig, usize)> = map
        .softrigs
        .values()
        .filter_map(|rig| {
            let name = rig.name.to_lowercase();
            let base = base_sequence.to_lowercase();

            if name.starts_with(&base) {
                let suffix = &rig.name[base.len()..];
                let num = suffix
                    .trim_start_matches(|c: char| !c.is_ascii_digit())
                    .parse::<usize>()
                    .unwrap_or(0);
                Some((rig, num))
            } else {
                None
            }
        })
        .collect();

    matched_rigs.sort_by_key(|(_, num)| *num);

    // for (rig, index) in &matched_rigs {
    //     println!("{} {}", rig.name, index);
    // }

    let mut forward_textures = Vec::new();
    let frames_per_transition = 4;

    match matched_rigs.len() {
        0 => {
            // Nothing matched
            let mut texture = Texture::alloc(size as usize, size as usize);
            let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
            stack.render_geometry(&mut texture, map, assets, false);
            forward_textures.push(texture);
        }
        1 => {
            // Only one rig
            let rig = matched_rigs[0].0;
            let mut temp_map = map.geometry_clone();
            temp_map.editing_rig = Some(rig.id);
            temp_map.softrigs.insert(rig.id, rig.clone());

            let mut texture = Texture::alloc(size as usize, size as usize);
            let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
            stack.render_geometry(&mut texture, &temp_map, assets, false);
            forward_textures.push(texture);
        }
        _ => {
            // Interpolate between rig pairs
            for i in 0..(matched_rigs.len() - 1) {
                let rig_a = matched_rigs[i].0;
                let rig_b = matched_rigs[i + 1].0;

                for f in 0..frames_per_transition {
                    let t = f as f32 / (frames_per_transition - 1) as f32;

                    let blended = SoftRigAnimator::blend_softrigs(rig_a, rig_b, t, map);

                    let mut temp_map = map.geometry_clone();
                    temp_map.editing_rig = Some(blended.id);
                    temp_map.softrigs.insert(blended.id, blended);

                    let mut texture = Texture::alloc(size as usize, size as usize);
                    let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
                    stack.render_geometry(&mut texture, &temp_map, assets, false);
                    forward_textures.push(texture);
                }
            }
        }
    }

    let ping_pong = true;

    let textures = if ping_pong && forward_textures.len() > 1 {
        let mut all = forward_textures.clone();
        // Skip last frame to avoid duplicate
        let mut reversed: Vec<_> = forward_textures[..forward_textures.len() - 1]
            .iter()
            .rev()
            .cloned()
            .collect();
        all.append(&mut reversed);
        all
    } else {
        forward_textures
    };

    Tile::from_textures(textures)
}
