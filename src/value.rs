use crate::PixelSource;
use std::fmt;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i32),
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Str(String),
    Id(Uuid),
    Source(PixelSource),
}

// Implement Display for Python-compatible string representation
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(val) => write!(f, "{}", val),
            Value::Int(val) => write!(f, "{}", val),
            Value::Float(val) => write!(f, "{:.6}", val), // Represent floats with 6 decimals
            Value::Vec2(val) => write!(f, "[{}, {}]", val[0], val[1]),
            Value::Vec3(val) => write!(f, "[{}, {}, {}]", val[0], val[1], val[2]),
            Value::Vec4(val) => write!(f, "[{}, {}, {}, {}]", val[0], val[1], val[2], val[3]),
            Value::Str(val) => write!(f, "{}", val.replace("'", "\\'")), // Escape single quotes
            Value::Id(val) => write!(f, "{}", val),
            Value::Source(val) => write!(f, "{:?}", val),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValueContainer {
    values: FxHashMap<String, Value>,
}

impl Default for ValueContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueContainer {
    // Create a new, empty ValueContainer
    pub fn new() -> Self {
        ValueContainer {
            values: FxHashMap::default(),
        }
    }

    // Add or update a value
    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
    }

    // Get a value by key
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    // Getters for specific value types by key
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(|v| {
            if let Value::Bool(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_int(&self, key: &str) -> Option<i32> {
        self.values.get(key).and_then(|v| {
            if let Value::Int(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_float(&self, key: &str) -> Option<f32> {
        self.values.get(key).and_then(|v| {
            if let Value::Float(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_float_default(&self, key: &str, def: f32) -> f32 {
        self.values
            .get(key)
            .map(|v| if let Value::Float(val) = v { *val } else { def })
            .unwrap_or(def)
    }

    pub fn get_vec2(&self, key: &str) -> Option<[f32; 2]> {
        self.values.get(key).and_then(|v| {
            if let Value::Vec2(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_vec3(&self, key: &str) -> Option<[f32; 3]> {
        self.values.get(key).and_then(|v| {
            if let Value::Vec3(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_vec4(&self, key: &str) -> Option<[f32; 4]> {
        self.values.get(key).and_then(|v| {
            if let Value::Vec4(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.values.get(key).and_then(|v| {
            if let Value::Str(ref val) = v {
                Some(val.as_str())
            } else {
                None
            }
        })
    }

    pub fn get_id(&self, key: &str) -> Option<Uuid> {
        self.values.get(key).and_then(|v| {
            if let Value::Id(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_source(&self, key: &str) -> Option<&PixelSource> {
        self.values.get(key).and_then(|v| {
            if let Value::Source(ref val) = v {
                Some(val)
            } else {
                None
            }
        })
    }

    // Remove a value by key
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.values.remove(key)
    }

    // Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    // Get all values
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.values.values()
    }
}

// Implement Display for ValueContainer
impl fmt::Display for ValueContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.values {
            writeln!(f, "{}: {}", key, value)?;
        }
        Ok(())
    }
}
