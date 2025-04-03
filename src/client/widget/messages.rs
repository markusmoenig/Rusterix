use crate::{client::draw2d, Assets, Rect};
use draw2d::Draw2D;
use theframework::prelude::*;

pub struct MessagesWidget {
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub font: Option<fontdue::Font>,
    pub font_size: f32,
    pub messages: Vec<String>,
    pub draw2d: Draw2D,
    pub spacing: f32,
}

impl Default for MessagesWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl MessagesWidget {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            font: None,
            font_size: 20.0,
            messages: vec![],
            draw2d: Draw2D::default(),
            spacing: 1.0,
        }
    }

    pub fn init(&mut self, assets: &Assets) {
        let mut font_name = String::new();
        if let Ok(table) = self.toml_str.parse::<toml::Table>() {
            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("font") {
                    if let Some(v) = value.as_str() {
                        font_name = v.into();
                    }
                }
                if let Some(value) = ui.get("font_size") {
                    if let Some(v) = value.as_float() {
                        self.font_size = v as f32;
                    }
                }
                if let Some(value) = ui.get("spacing") {
                    if let Some(v) = value.as_float() {
                        self.spacing = v as f32;
                    }
                }
            }
        }

        if let Some(font) = assets.fonts.get(&font_name) {
            self.font = Some(font.clone());
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _assets: &Assets,
        messages: Vec<(Option<u32>, Option<u32>, u32, String)>,
    ) {
        // Append new messages
        for message in &messages {
            self.messages.push(message.3.clone());
        }

        // Purge the messages which are scrolled out of scope
        let max_messages = 100;
        if self.messages.len() > max_messages {
            let excess = self.messages.len() - max_messages;
            self.messages.drain(0..excess);
        }

        // Draw bottom up
        if let Some(font) = &self.font {
            let stride = buffer.stride();
            let mut y = self.rect.y + self.rect.height - self.font_size.ceil();

            for message in self.messages.iter().rev() {
                if y + self.font_size < self.rect.y {
                    break;
                }

                let tuple = (
                    self.rect.x as isize,
                    y.floor() as isize,
                    self.rect.width as isize,
                    self.font_size as isize,
                );

                self.draw2d.text_rect_blend_safe(
                    buffer.pixels_mut(),
                    &tuple,
                    stride,
                    font,
                    self.font_size,
                    message,
                    &[128, 128, 128, 255],
                    draw2d::TheHorizontalAlign::Left,
                    draw2d::TheVerticalAlign::Center,
                    &(
                        self.rect.x as isize,
                        self.rect.y as isize,
                        self.rect.width as isize,
                        self.rect.height as isize,
                    ),
                );

                y -= self.font_size + self.spacing;
            }
        }
    }
}
