use crate::Value;
use theframework::prelude::*;

/// Messages to the Region
#[derive(Debug)]
pub enum RegionMessage {
    /// Register a local player (which receives user based events).
    /// RegionInstanceId, PlayerId
    RegisterPlayer(u32, u32),
    /// An event
    Event(u32, String, Value),
    /// A user event
    UserEvent(u32, String, Value),
    /// A user action
    UserAction(u32, EntityAction),
    /// Entity updates for a given region instance
    EntitiesUpdate(u32, Vec<Vec<u8>>),
    /// Item updates for a given region instance
    ItemsUpdate(u32, Vec<Vec<u8>>),
    /// Remove the given item from the Region
    RemoveItem(u32, u32),
    /// Log Message
    LogMessage(String),
    /// Time event of a Region
    Time(u32, TheTime),
    /// Tell: RegionId, SenderId_entity, SenderId_item, ReceiverId, Message
    Tell(u32, Option<u32>, Option<u32>, u32, String),
    /// Stop processing and quit
    Quit,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum EntityAction {
    #[default]
    Off,
    Left,
    Forward,
    Right,
    Backward,
    /// Sleep until the given tick and switch back to the given action
    SleepAndSwitch(i64, Box<EntityAction>),
    /// User: Distance, Speed, Max Min Sleep. System: State, Target
    RandomWalk(f32, f32, i32, i32, Vec2<f32>),
    /// User: Distance, Speed, Max Min Sleep. System: State, Target
    RandomWalkInSector(f32, f32, i32, i32, Vec2<f32>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum PlayerCamera {
    #[default]
    D2,
    D3Iso,
    D3FirstP,
}

use std::str::FromStr;
impl FromStr for EntityAction {
    type Err = ();

    /// Converts a `&str` to an `EntityAction`.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(EntityAction::Off),
            "left" => Ok(EntityAction::Left),
            "forward" => Ok(EntityAction::Forward),
            "right" => Ok(EntityAction::Right),
            "backward" => Ok(EntityAction::Backward),
            _ => Err(()), // Return an error for invalid values
        }
    }
}

use std::convert::TryFrom;
impl TryFrom<i32> for EntityAction {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EntityAction::Left),
            1 => Ok(EntityAction::Forward),
            2 => Ok(EntityAction::Right),
            3 => Ok(EntityAction::Backward),
            _ => Err("Invalid value for EntityAction"),
        }
    }
}
