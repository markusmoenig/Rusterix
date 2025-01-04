use crate::Value;
use theframework::prelude::*;

/// Messages to the Region
pub enum RegionMessage {
    /// Register a local player (which receives user based events).
    RegisterPlayer(u32),
    /// An event
    Event(u32, String, Value),
    /// A user event
    UserEvent(u32, String, Value),
    /// A user action
    UserAction(u32, EntityAction),
    /// Entity updates for a given region
    EntitiesUpdate(Vec<Vec<u8>>),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityAction {
    #[default]
    Off,
    West,
    North,
    East,
    South,
}

impl EntityAction {
    /// Converts an `i32` to an `EntityAction`
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(EntityAction::Off),
            1 => Some(EntityAction::West),
            2 => Some(EntityAction::North),
            3 => Some(EntityAction::East),
            4 => Some(EntityAction::South),
            _ => None, // Return None for invalid values
        }
    }
}

use std::convert::TryFrom;

impl TryFrom<i32> for EntityAction {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EntityAction::West),
            1 => Ok(EntityAction::North),
            2 => Ok(EntityAction::East),
            3 => Ok(EntityAction::South),
            _ => Err("Invalid value for EntityAction"),
        }
    }
}
