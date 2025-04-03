pub mod game;
pub mod messages;
pub mod screen;

use crate::prelude::Rect;

pub struct Widget {
    pub rect: Rect,
    pub action: String,
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            action: String::new(),
        }
    }
}
