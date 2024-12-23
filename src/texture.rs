use crate::IntoDataInput;
use std::io::Cursor;
use theframework::prelude::*;

/// Sample mode for texture sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleMode {
    /// Nearest-neighbor sampling
    Nearest,
    /// Linear interpolation sampling
    Linear,
}

/// The repeat mode for texture sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    /// Clamps UVs to [0, 1] (the default)
    ClampXY,
    /// Repeats texture in both X and Y
    RepeatXY,
    /// Repeats texture only in X
    RepeatX,
    /// Repeats texture only in Y
    RepeatY,
}

/// Textures contain RGBA [u8;4] pixels.
#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Clone, Debug)]
pub struct Texture {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl Default for Texture {
    fn default() -> Self {
        Self::white()
    }
}

impl Texture {
    /// Creates a new texture with the given width, height, and data
    pub fn new(data: Vec<u8>, width: usize, height: usize) -> Self {
        assert_eq!(data.len(), width * height * 4, "Invalid texture data size.");
        Texture {
            data,
            width,
            height,
        }
    }

    /// Creates a default 100x100 checkerboard texture
    pub fn checkerboard(size: usize, square_size: usize) -> Self {
        let width = size;
        let height = size;
        let mut data = vec![0; width * height * 4]; // Initialize texture data

        for y in 0..height {
            for x in 0..width {
                let is_white = ((x / square_size) + (y / square_size)) % 2 == 0;
                let color = if is_white {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                };

                let idx = (y * width + x) * 4;
                data[idx..idx + 4].copy_from_slice(&color);
            }
        }

        Texture {
            data,
            width,
            height,
        }
    }

    /// Creates a texture filled with a single color (1x1 texture)
    pub fn from_color(color: [u8; 4]) -> Self {
        Texture {
            data: color.to_vec(),
            width: 1,
            height: 1,
        }
    }

    /// Creates a texture filled with a white color (1x1 texture)
    pub fn white() -> Self {
        Texture {
            data: vec![255, 255, 255, 255],
            width: 1,
            height: 1,
        }
    }

    /// Loads a texture from an image file at the given path.
    pub fn from_image(input: impl IntoDataInput) -> Self {
        // Load the image from the input source
        let data = input.load_data().expect("Failed to load data");
        let img = image::ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .expect("Failed to read image format")
            .decode()
            .expect("Failed to decode the image");

        // Convert to RGBA8 format
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        // Flatten the image data into a Vec<u8>
        let data = rgba_img.into_raw();

        Texture {
            data,
            width: width as usize,
            height: height as usize,
        }
    }

    /// Loads a texture from an image file at the given path (if available).
    pub fn from_image_safe(input: impl IntoDataInput) -> Option<Self> {
        // Try to load the image from the input source
        let data = input.load_data().ok()?;
        let img = image::ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .ok()? // Early return on format guessing failure
            .decode()
            .ok()?; // Early return on decoding failure

        // Convert to RGBA8 format
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        // Flatten the image data into a Vec<u8>
        let data = rgba_img.into_raw();

        Some(Texture {
            data,
            width: width as usize,
            height: height as usize,
        })
    }

    /// Samples the texture using the specified sampling and repeat mode
    pub fn sample(
        &self,
        mut u: f32,
        mut v: f32,
        sample_mode: SampleMode,
        repeat_mode: RepeatMode,
    ) -> [u8; 4] {
        match repeat_mode {
            RepeatMode::ClampXY => {
                u = u.clamp(0.0, 1.0);
                v = v.clamp(0.0, 1.0);
            }
            RepeatMode::RepeatXY => {
                u = u - u.floor(); // Wraps in both X and Y
                v = v - v.floor();
            }
            RepeatMode::RepeatX => {
                u = u - u.floor(); // Wraps only in X
                v = v.clamp(0.0, 1.0);
            }
            RepeatMode::RepeatY => {
                u = u.clamp(0.0, 1.0);
                v = v - v.floor(); // Wraps only in Y
            }
        }
        match sample_mode {
            SampleMode::Nearest => self.sample_nearest(u, v),
            SampleMode::Linear => self.sample_linear(u, v),
        }
    }

    /// Samples the texture at given UV coordinates.
    pub fn sample_nearest(&self, u: f32, v: f32) -> [u8; 4] {
        // Map UV coordinates to pixel indices
        let tex_x = (u * (self.width as f32 - 1.0)).round() as usize;
        let tex_y = (v * (self.height as f32 - 1.0)).round() as usize;

        // Retrieve the color from the texture
        let idx = (tex_y * self.width + tex_x) * 4;
        [
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]
    }

    /// Samples the texture using linear interpolation at the given UV coordinates
    pub fn sample_linear(&self, u: f32, v: f32) -> [u8; 4] {
        // Clamp UV coordinates to [0, 1]
        // let u = u.clamp(0.0, 1.0);
        // let v = v.clamp(0.0, 1.0);

        // Map UV coordinates to floating-point pixel coordinates
        let x = u * (self.width as f32 - 1.0);
        let y = v * (self.height as f32 - 1.0);

        // Calculate integer pixel indices and fractional offsets
        let x0 = x.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1); // Clamp to texture bounds
        let y0 = y.floor() as usize;
        let y1 = (y0 + 1).min(self.height - 1); // Clamp to texture bounds

        let dx = x - x.floor(); // Fractional part of x
        let dy = y - y.floor(); // Fractional part of y

        // Sample the four texels
        let idx00 = (y0 * self.width + x0) * 4;
        let idx10 = (y0 * self.width + x1) * 4;
        let idx01 = (y1 * self.width + x0) * 4;
        let idx11 = (y1 * self.width + x1) * 4;

        let c00 = &self.data[idx00..idx00 + 4];
        let c10 = &self.data[idx10..idx10 + 4];
        let c01 = &self.data[idx01..idx01 + 4];
        let c11 = &self.data[idx11..idx11 + 4];

        // Interpolate the colors
        let mut result = [0u8; 4];
        for i in 0..4 {
            let v00 = c00[i] as f32;
            let v10 = c10[i] as f32;
            let v01 = c01[i] as f32;
            let v11 = c11[i] as f32;

            // Bilinear interpolation formula
            let v0 = v00 + dx * (v10 - v00); // Interpolate along x at y0
            let v1 = v01 + dx * (v11 - v01); // Interpolate along x at y1
            let v = v0 + dy * (v1 - v0); // Interpolate along y

            result[i] = v.round() as u8;
        }

        result
    }
}
