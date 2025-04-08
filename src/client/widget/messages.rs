use crate::{Assets, Pixel, Rect, client::draw2d};
use draw2d::Draw2D;
use theframework::prelude::*;

pub struct MessagesWidget {
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub font: Option<fontdue::Font>,
    pub font_size: f32,
    pub messages: Vec<(String, Pixel)>,
    pub draw2d: Draw2D,
    pub spacing: f32,
    pub table: toml::Table,
    pub top_down: bool,
    pub default_color: Pixel,
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
            table: toml::Table::default(),
            top_down: false,
            default_color: [170, 170, 170, 255],
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
                if let Some(value) = ui.get("top_down") {
                    if let Some(v) = value.as_bool() {
                        self.top_down = v;
                    }
                }
                if let Some(value) = ui.get("default") {
                    if let Some(v) = value.as_str() {
                        self.default_color = self.hex_to_rgba_u8(v);
                    }
                }
            }
            self.table = table;
        }

        if let Some(font) = assets.fonts.get(&font_name) {
            self.font = Some(font.clone());
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _assets: &Assets,
        messages: Vec<crate::server::Message>,
    ) {
        let width = buffer.dim().width;
        let height = buffer.dim().height;

        // Append new messages
        for (_, _, _, message, category) in &messages {
            let mut color = self.default_color;
            if let Some(ui) = self.table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get(category) {
                    if let Some(v) = value.as_str() {
                        color = self.hex_to_rgba_u8(v);
                    }
                }
            }
            self.messages.push((message.clone(), color));
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
            let mut y = if self.top_down {
                self.rect.y
            } else {
                self.rect.y + self.rect.height - self.font_size.ceil()
            };

            for (message, color) in self.messages.iter().rev() {
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
                    color,
                    draw2d::TheHorizontalAlign::Left,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, width as isize, height as isize),
                );

                if self.top_down {
                    y += self.font_size + self.spacing;
                } else {
                    y -= self.font_size + self.spacing;
                }
            }
        }
    }

    /// Converts a hex color string to a [u8; 4] (RGBA).
    /// Accepts "#RRGGBB" or "#RRGGBBAA" formats.
    fn hex_to_rgba_u8(&self, hex: &str) -> [u8; 4] {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            6 => match (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                (Ok(r), Ok(g), Ok(b)) => [r, g, b, 255],
                _ => [255, 255, 255, 255],
            },
            8 => match (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
                u8::from_str_radix(&hex[6..8], 16),
            ) {
                (Ok(r), Ok(g), Ok(b), Ok(a)) => [r, g, b, a],
                _ => [255, 255, 255, 255],
            },
            _ => [255, 255, 255, 255],
        }
    }
}
