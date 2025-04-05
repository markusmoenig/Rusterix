pub mod game;
pub mod messages;
pub mod screen;
pub mod text;

use crate::prelude::Rect;

/// Used right now for button widgets
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
