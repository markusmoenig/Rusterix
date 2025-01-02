use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i32),
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Str(String),
}

// Implement Display for Python-compatible string representation
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(val) => write!(f, "{}", val),
            Value::Float(val) => write!(f, "{:.6}", val), // Represent floats with 6 decimals
            Value::Vec2(val) => write!(f, "[{}, {}]", val[0], val[1]),
            Value::Vec3(val) => write!(f, "[{}, {}, {}]", val[0], val[1], val[2]),
            Value::Vec4(val) => write!(f, "[{}, {}, {}, {}]", val[0], val[1], val[2], val[3]),
            Value::Str(val) => write!(f, "'{}'", val.replace("'", "\\'")), // Escape single quotes
        }
    }
}
