use crate::server::py_fn::*;
use crate::{
    Assets, Currencies, Currency, Entity, EntityAction, Item, Map, MapMini, PixelSource,
    PlayerCamera, RegionData, RegionInstance, RegionMessage, Value, ValueContainer,
};
use crossbeam_channel::{Receiver, Sender, select, tick, unbounded};
use rand::*;
use ref_thread_local::{RefThreadLocal, ref_thread_local};

use rustpython::vm::*;
use std::sync::{Arc, Mutex, OnceLock};
use theframework::prelude::{FxHashMap, FxHashSet, TheTime, Uuid};
use vek::num_traits::zero;

// Local thread globals which can be accessed from both Rust and Rhai
ref_thread_local! {
    pub static managed REGIONDATA      : Vec<RegionData> = vec![];

    pub static managed CURR_INST        : usize = 0;
}

pub struct RegionPool {
    /// Send messages to this pool
    pub to_sender: Sender<RegionMessage>,
    /// Local receiver
    to_receiver: Receiver<RegionMessage>,

    /// Send messages from this pool
    from_sender: Sender<RegionMessage>,
    /// Local receiver
    pub from_receiver: Receiver<RegionMessage>,
}

impl Default for RegionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionPool {
    pub fn new() -> Self {
        let (to_sender, to_receiver) = unbounded::<RegionMessage>();
        let (from_sender, from_receiver) = unbounded::<RegionMessage>();
        Self {
            to_receiver,
            to_sender,
            from_receiver,
            from_sender,
        }
    }
}
