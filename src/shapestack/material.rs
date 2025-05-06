use std::str::FromStr;
use theframework::prelude::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaterialRole {
    Matte,
    Glossy,
    Metallic,
    Transparent,
    Emissive,
}

use MaterialRole::*;

impl MaterialRole {
    pub fn to_u8(&self) -> u8 {
        match self {
            MaterialRole::Matte => 0,
            MaterialRole::Glossy => 1,
            MaterialRole::Metallic => 2,
            MaterialRole::Transparent => 3,
            MaterialRole::Emissive => 4,
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(MaterialRole::Matte),
            1 => Some(MaterialRole::Glossy),
            2 => Some(MaterialRole::Metallic),
            3 => Some(MaterialRole::Transparent),
            4 => Some(MaterialRole::Emissive),
            _ => None,
        }
    }
}

impl FromStr for MaterialRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Matte" => Ok(Matte),
            "Glossy" => Ok(Glossy),
            "Metallic" => Ok(Metallic),
            "Transparent" => Ok(Transparent),
            "Emissive" => Ok(Emissive),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub role: MaterialRole,
    pub value: f32,
}

impl Material {
    pub fn new(role: MaterialRole, value: f32) -> Self {
        Self { role, value }
    }
}
