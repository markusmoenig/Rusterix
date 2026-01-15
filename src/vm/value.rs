use crate::value::Value;
use std::ops::{Add, Div, Mul, Neg, Sub};
use vek::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct VMValue {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub string: Option<String>,
}

impl VMValue {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            string: None,
        }
    }

    pub fn broadcast(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            string: None,
        }
    }

    pub fn zero() -> Self {
        Self::broadcast(0.0)
    }

    pub fn from_bool(v: bool) -> Self {
        Self::broadcast(if v { 1.0 } else { 0.0 })
    }

    pub fn from_vec3(v: Vec3<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            string: None,
        }
    }

    pub fn to_vec3(&self) -> Vec3<f32> {
        Vec3::new(self.x, self.y, self.z)
    }

    pub fn from_string<S: Into<String>>(s: S) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            string: Some(s.into()),
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        self.string.as_deref()
    }

    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::NoValue => VMValue::zero(),
            Value::Bool(b) => VMValue::broadcast(if *b { 1.0 } else { 0.0 }),
            Value::Int(i) => VMValue::broadcast(*i as f32),
            Value::UInt(i) => VMValue::broadcast(*i as f32),
            Value::Int64(i) => VMValue::broadcast(*i as f32),
            Value::Float(f) => VMValue::broadcast(*f),
            Value::Vec2(v) => VMValue::new(v[0], v[1], 0.0),
            Value::Vec3(v) => VMValue::new(v[0], v[1], v[2]),
            Value::Vec4(v) => VMValue::new(v[0], v[1], v[2]),
            Value::Str(s) => VMValue::from_string(s.clone()),
            _ => VMValue::zero(),
        }
    }

    /// Convert into a generic runtime Value.
    pub fn to_value(&self) -> Value {
        if let Some(s) = self.as_string() {
            Value::Str(s.to_string())
        } else if self.x == self.y && self.x == self.z {
            Value::Float(self.x)
        } else {
            Value::Vec3([self.x, self.y, self.z])
        }
    }

    pub fn is_truthy(&self) -> bool {
        if let Some(s) = &self.string {
            !s.is_empty()
        } else {
            self.x != 0.0 || self.y != 0.0 || self.z != 0.0
        }
    }

    pub fn magnitude(&self) -> f32 {
        self.to_vec3().magnitude()
    }

    pub fn map<F: Fn(f32) -> f32>(&self, f: F) -> Self {
        VMValue::new(f(self.x), f(self.y), f(self.z))
    }

    pub fn map2<F: Fn(f32, f32) -> f32>(&self, other: VMValue, f: F) -> Self {
        VMValue::new(f(self.x, other.x), f(self.y, other.y), f(self.z, other.z))
    }

    pub fn dot(&self, other: VMValue) -> f32 {
        self.to_vec3().dot(other.to_vec3())
    }

    pub fn cross(&self, other: VMValue) -> Self {
        VMValue::from_vec3(self.to_vec3().cross(other.to_vec3()))
    }
}

impl Add for VMValue {
    type Output = VMValue;

    fn add(self, rhs: VMValue) -> Self::Output {
        match (self.string, rhs.string) {
            (Some(a), Some(b)) => VMValue::from_string(format!("{a}{b}")),
            _ => VMValue::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z),
        }
    }
}

impl Sub for VMValue {
    type Output = VMValue;

    fn sub(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul for VMValue {
    type Output = VMValue;

    fn mul(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

impl Div for VMValue {
    type Output = VMValue;

    fn div(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x / rhs.x, self.y / rhs.y, self.z / rhs.z)
    }
}

impl Neg for VMValue {
    type Output = VMValue;

    fn neg(self) -> Self::Output {
        VMValue::new(-self.x, -self.y, -self.z)
    }
}
