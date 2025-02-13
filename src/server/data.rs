use crate::{Entity, Item, Light, LightType, PixelSource, Value};
use theframework::prelude::*;
use toml::Table;

/// Apply toml data to an Entity.
pub fn apply_entity_data(entity: &mut Entity, toml: &str) {
    match toml.parse::<Table>() {
        Ok(map) => {
            for (attr, v) in map.iter() {
                if attr == "attributes" {
                    if let Some(values) = v.as_table() {
                        for (key, value) in values {
                            if let Some(value) = value.as_float() {
                                entity.set_attribute(key, crate::Value::Float(value as f32));
                            } else if let Some(value) = value.as_integer() {
                                entity.set_attribute(key, crate::Value::Int(value as i32));
                            } else if let Some(value) = value.as_str() {
                                if key == "tile_id" {
                                    if let Ok(uuid) = Uuid::parse_str(value) {
                                        entity.set_attribute(
                                            "source",
                                            Value::Source(PixelSource::TileId(uuid)),
                                        );
                                    }
                                } else {
                                    entity.set_attribute(key, crate::Value::Str(value.to_string()));
                                }
                            }
                        }
                    }
                } else if attr == "light" {
                    let light = Light::new(LightType::Point);
                    entity.set_attribute("light", crate::Value::Light(light));
                }
            }
        }
        Err(err) => {
            println!("error {:?}", err);
        }
    }
}

/// Apply toml data to an Item.
pub fn apply_item_data(item: &mut Item, toml: &str) {
    match toml.parse::<Table>() {
        Ok(map) => {
            for (attr, v) in map.iter() {
                if attr == "attributes" {
                    if let Some(values) = v.as_table() {
                        for (key, value) in values {
                            if let Some(value) = value.as_float() {
                                item.set_attribute(key, crate::Value::Float(value as f32));
                            } else if let Some(value) = value.as_integer() {
                                item.set_attribute(key, crate::Value::Int(value as i32));
                            } else if let Some(value) = value.as_str() {
                                if key == "tile_id" {
                                    if let Ok(uuid) = Uuid::parse_str(value) {
                                        item.set_attribute(
                                            "source",
                                            Value::Source(PixelSource::TileId(uuid)),
                                        );
                                    }
                                } else {
                                    item.set_attribute(key, crate::Value::Str(value.to_string()));
                                }
                            }
                        }
                    }
                } else if attr == "light" {
                    let light = Light::new(LightType::Point);
                    item.set_attribute("light", crate::Value::Light(light));
                }
            }
        }
        Err(err) => {
            println!("error {:?}", err);
        }
    }
}
