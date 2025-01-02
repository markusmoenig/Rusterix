use crate::Value;
// use theframework::prelude::FxHashMap;

/// Messages from Python to the regions.
pub enum ServerMessage {
    /// Register a local player (which receives user based events).
    RegisterPlayer(u32),
    /// An event
    Event(u32, String, Value),
}
