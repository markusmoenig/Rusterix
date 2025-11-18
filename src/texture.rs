use crate::{IntoDataInput, MaterialProfile};
use std::io::Cursor;
use theframework::prelude::*;
use vek::Vec3;

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Sample mode for texture sampling.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
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
    /// Optional sub-texture that stores a generated normal map (RGBA8; XYZ in 0..255, A unused)
    pub normal_map: Option<Box<Texture>>,
    /// Optional sub-texture for per-pixel materials (RGBA8): R=roughness, G=metallic, B=opacity, A=unused
    pub material_map: Option<Box<Texture>>,
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
            normal_map: None,
            material_map: None,
        }
    }

    /// Creates a new texture with the given width, height, and allocates the data.
    pub fn alloc(width: usize, height: usize) -> Self {
        Texture {
            data: vec![0; width * height * 4],
            width,
            height,
            normal_map: None,
            material_map: None,
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
                    [128, 128, 128, 255]
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
            normal_map: None,
            material_map: None,
        }
    }

    /// Creates a texture filled with a single color (1x1 texture)
    pub fn from_color(color: [u8; 4]) -> Self {
        Texture {
            data: color.to_vec(),
            width: 1,
            height: 1,
            normal_map: None,
            material_map: None,
        }
    }

    /// Creates a texture filled with a white color (1x1 texture)
    pub fn white() -> Self {
        Texture {
            data: vec![255, 255, 255, 255],
            width: 1,
            height: 1,
            normal_map: None,
            material_map: None,
        }
    }

    /// Creates a texture filled with a black color (1x1 texture)
    pub fn black() -> Self {
        Texture {
            data: vec![0, 0, 0, 255],
            width: 1,
            height: 1,
            normal_map: None,
            material_map: None,
        }
    }

    pub fn from_rgbabuffer(buffer: &TheRGBABuffer) -> Self {
        Texture {
            data: buffer.pixels().to_vec(),
            width: buffer.dim().width as usize,
            height: buffer.dim().height as usize,
            normal_map: None,
            material_map: None,
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
            normal_map: None,
            material_map: None,
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
            normal_map: None,
            material_map: None,
        })
    }

    /// Samples the texture using the specified sampling and repeat mode
    #[inline(always)]
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

    /// Samples the texture and optionally perturbs a provided normal using this texture's normal_map.
    #[inline(always)]
    pub fn sample_with_normal(
        &self,
        u: f32,
        v: f32,
        sample_mode: SampleMode,
        repeat_mode: RepeatMode,
        mut normal: Option<&mut Vec3<f32>>,
        normal_strength: f32,
    ) -> [u8; 4] {
        // Same UV handling and color sampling as sample()
        let color = {
            let (mut uu, mut vv) = (u, v);
            match repeat_mode {
                RepeatMode::ClampXY => {
                    uu = uu.clamp(0.0, 1.0);
                    vv = vv.clamp(0.0, 1.0);
                }
                RepeatMode::RepeatXY => {
                    uu -= uu.floor();
                    vv -= vv.floor();
                }
                RepeatMode::RepeatX => {
                    uu -= uu.floor();
                    vv = vv.clamp(0.0, 1.0);
                }
                RepeatMode::RepeatY => {
                    uu = uu.clamp(0.0, 1.0);
                    vv -= vv.floor();
                }
            }
            match sample_mode {
                SampleMode::Nearest => self.sample_nearest(uu, vv),
                SampleMode::Linear => self.sample_linear(uu, vv),
            }
        };

        if let (Some(n_ref), Some(norm_tex)) = (normal.as_deref_mut(), self.normal_map.as_deref()) {
            // Decode tangent-space normal [-1,1]
            let n_rgba = norm_tex.sample(u, v, sample_mode, repeat_mode);
            let mut nx = (n_rgba[0] as f32 / 255.0) * 2.0 - 1.0;
            let mut ny = (n_rgba[1] as f32 / 255.0) * 2.0 - 1.0;
            let nz = (n_rgba[2] as f32 / 255.0) * 2.0 - 1.0;

            // Apply strength at runtime: scale tangent XY, renormalize
            nx *= normal_strength;
            ny *= normal_strength;
            let ts = Vec3::new(nx, ny, nz).normalized();

            // Build TBN from current normal and transform
            let n = (*n_ref).normalized();
            let helper = if n.x.abs() < 0.5 {
                Vec3::new(1.0, 0.0, 0.0)
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };
            let t = n.cross(helper).normalized();
            let b = n.cross(t);
            *n_ref = (t * ts.x + b * ts.y + n * ts.z).normalized();
        }

        color
    }

    /// Samples the texture using the specified sampling and repeat mode
    pub fn sample_blur(
        &self,
        mut u: f32,
        mut v: f32,
        sample_mode: SampleMode,
        repeat_mode: RepeatMode,
        blur_strength: f32,
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
            SampleMode::Nearest => {
                if blur_strength == 0.0 {
                    self.sample_nearest(u, v)
                } else {
                    self.sample_nearest_blur(u, v, blur_strength)
                }
            }

            SampleMode::Linear => self.sample_linear(u, v),
        }
    }

    // Samples the texture at given UV coordinates.
    // pub fn sample_nearest(&self, u: f32, v: f32) -> [u8; 4] {
    //     // Map UV coordinates to pixel indices
    //     let tex_x = (u * (self.width as f32 - 1.0)).round() as usize;
    //     let tex_y = (v * (self.height as f32 - 1.0)).round() as usize;

    //     // Retrieve the color from the texture
    //     let idx = (tex_y * self.width + tex_x) * 4;
    //     [
    //         self.data[idx],
    //         self.data[idx + 1],
    //         self.data[idx + 2],
    //         self.data[idx + 3],
    //     ]
    // }
    // #[inline(always)]
    // pub fn sample_nearest(&self, u: f32, v: f32) -> [u8; 4] {
    //     let mut tx = (u * self.width as f32 + 0.5).floor() as usize;
    //     let mut ty = (v * self.height as f32 + 0.5).floor() as usize;

    //     tx = tx.clamp(0, self.width - 1);
    //     ty = ty.clamp(0, self.height - 1);

    //     let idx = (ty * self.width + tx) * 4;
    //     [
    //         self.data[idx],
    //         self.data[idx + 1],
    //         self.data[idx + 2],
    //         self.data[idx + 3],
    //     ]
    // }
    //
    #[inline(always)]
    pub fn sample_nearest(&self, u: f32, v: f32) -> [u8; 4] {
        // Properly map [0.0, 1.0] to texel centers
        let mut tx = (u * (self.width as f32 - 1.0)).round() as usize;
        let mut ty = (v * (self.height as f32 - 1.0)).round() as usize;

        // Clamp to prevent out-of-bounds
        tx = tx.clamp(0, self.width - 1);
        ty = ty.clamp(0, self.height - 1);

        let idx = (ty * self.width + tx) * 4;
        [
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]
    }

    /// Samples the texture at given UV coordinates.
    #[inline(always)]
    pub fn sample_nearest_blur(&self, u: f32, v: f32, blur_strength: f32) -> [u8; 4] {
        // Clamp blur_strength to [0, 1]
        let blur_strength = blur_strength.clamp(0.0, 1.0);

        // Map UV coordinates to pixel indices
        let mut tx = (u * self.width as f32 + 0.5).floor() as i32;
        let mut ty = (v * self.height as f32 + 0.5).floor() as i32;

        // Clamp texel coordinates to texture bounds
        if tx < 0 {
            tx = 0;
        } else if tx >= self.width as i32 {
            tx = self.width as i32 - 1;
        }
        if ty < 0 {
            ty = 0;
        } else if ty >= self.height as i32 {
            ty = self.height as i32 - 1;
        }

        // If blur_strength is 0, fall back to pure nearest sampling
        if blur_strength == 0.0 {
            let idx = (ty as usize * self.width + tx as usize) * 4;
            return [
                self.data[idx],
                self.data[idx + 1],
                self.data[idx + 2],
                self.data[idx + 3],
            ];
        }

        // Define a 3x3 kernel for blurring
        let offsets = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (0, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];

        // Accumulate color values from neighboring texels
        let mut result = [0.0f32; 4];
        let mut total_weight = 0.0f32;

        for &(dx, dy) in &offsets {
            let nx = (tx + dx).clamp(0, self.width as i32 - 1) as usize;
            let ny = (ty + dy).clamp(0, self.height as i32 - 1) as usize;

            // Calculate weight based on distance from center
            let distance = ((dx.abs() + dy.abs()) as f32).max(1.0); // Avoid division by zero
            let weight = (1.0 / distance) * blur_strength;

            // Retrieve the color from the texture
            let idx = (ny * self.width + nx) * 4;
            let color = [
                self.data[idx] as f32,
                self.data[idx + 1] as f32,
                self.data[idx + 2] as f32,
                self.data[idx + 3] as f32,
            ];

            // Accumulate weighted color
            for i in 0..4 {
                result[i] += color[i] * weight;
            }
            total_weight += weight;
        }

        // Normalize the result by total weight
        for item in &mut result {
            *item /= total_weight;
        }

        // Convert back to u8
        [
            result[0].round() as u8,
            result[1].round() as u8,
            result[2].round() as u8,
            result[3].round() as u8,
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

    /// Returns a new Texture resized to the specified width and height using nearest-neighbor sampling.
    pub fn resized(&self, new_width: usize, new_height: usize) -> Self {
        let mut new_data = vec![0; new_width * new_height * 4];
        let scale_x = self.width as f32 / new_width as f32;
        let scale_y = self.height as f32 / new_height as f32;

        for y in 0..new_height {
            for x in 0..new_width {
                let mut src_x = (x as f32 * scale_x) as usize;
                if src_x >= self.width {
                    src_x = self.width - 1;
                }

                let mut src_y = (y as f32 * scale_y) as usize;
                if src_y >= self.height {
                    src_y = self.height - 1;
                }

                let src_idx = (src_y * self.width + src_x) * 4;
                let dst_idx = (y * new_width + x) * 4;

                new_data[dst_idx..dst_idx + 4].copy_from_slice(&self.data[src_idx..src_idx + 4]);
            }
        }

        // Resize normal_map if present
        let resized_normal_map = self
            .normal_map
            .as_ref()
            .map(|nm| Box::new(nm.resized(new_width, new_height)));

        // Resize material_map if present
        let resized_material_map = self
            .material_map
            .as_ref()
            .map(|mm| Box::new(mm.resized(new_width, new_height)));

        Texture {
            data: new_data,
            width: new_width,
            height: new_height,
            normal_map: resized_normal_map,
            material_map: resized_material_map,
        }
    }

    /// Fills the entire texture with the specified color
    pub fn fill(&mut self, color: [u8; 4]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) * 4;
                self.data[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }

    /// Gets the pixel at the specified (x, y) position. Clamps to bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        [
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]
    }

    /// Sets the pixel at the specified (x, y) position. Clamps to bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        self.data[idx..idx + 4].copy_from_slice(&color);
    }

    /// Convert to an TheRGBABuffer
    pub fn to_rgba(&self) -> TheRGBABuffer {
        TheRGBABuffer::from(self.data.clone(), self.width as u32, self.height as u32)
    }

    /// Generates a normal-map subtexture from this texture's color data using a Sobel filter on luma.
    /// The resulting normals are encoded as RGBA8 where XYZ are mapped from [-1,1] to [0,255] and A is 255.
    ///
    /// `wrap`: if true, samples wrap at edges (tiles nicely); if false, clamps at borders.
    pub fn generate_normals(&mut self, wrap: bool) {
        let w = self.width as i32;
        let h = self.height as i32;
        let mut out = vec![0u8; (w as usize) * (h as usize) * 4];

        // Precompute luma (height) as f32 in [0,1]
        let mut height = vec![0.0f32; (w as usize) * (h as usize)];
        for y in 0..h {
            for x in 0..w {
                let idx = ((y as usize) * self.width + (x as usize)) * 4;
                let r = self.data[idx] as f32 / 255.0;
                let g = self.data[idx + 1] as f32 / 255.0;
                let b = self.data[idx + 2] as f32 / 255.0;
                // Perceptual luma
                let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                height[(y as usize) * self.width + (x as usize)] = l;
            }
        }

        // Helper to read height with wrap/clamp
        let sample_h = |xx: i32, yy: i32| -> f32 {
            let (mut sx, mut sy) = (xx, yy);
            if wrap {
                sx = ((sx % w) + w) % w;
                sy = ((sy % h) + h) % h;
            } else {
                if sx < 0 {
                    sx = 0;
                } else if sx >= w {
                    sx = w - 1;
                }
                if sy < 0 {
                    sy = 0;
                } else if sy >= h {
                    sy = h - 1;
                }
            }
            height[(sy as usize) * self.width + (sx as usize)]
        };

        // Sobel kernels
        for y in 0..h {
            for x in 0..w {
                let tl = sample_h(x - 1, y - 1);
                let tc = sample_h(x + 0, y - 1);
                let tr = sample_h(x + 1, y - 1);
                let cl = sample_h(x - 1, y + 0);
                let cr = sample_h(x + 1, y + 0);
                let bl = sample_h(x - 1, y + 1);
                let bc = sample_h(x + 0, y + 1);
                let br = sample_h(x + 1, y + 1);

                let gx = (-1.0 * tl)
                    + (0.0 * tc)
                    + (1.0 * tr)
                    + (-2.0 * cl)
                    + (0.0 * 0.0)
                    + (2.0 * cr)
                    + (-1.0 * bl)
                    + (0.0 * bc)
                    + (1.0 * br);

                let gy = (-1.0 * tl)
                    + (-2.0 * tc)
                    + (-1.0 * tr)
                    + (0.0 * cl)
                    + (0.0 * 0.0)
                    + (0.0 * cr)
                    + (1.0 * bl)
                    + (2.0 * bc)
                    + (1.0 * br);

                // Build normal; Z up, scale X/Y by strength
                let nx = -gx;
                let ny = -gy;
                let nz = 1.0;
                let len = (nx * nx + ny * ny + nz * nz).sqrt();
                let (nx, ny, nz) = if len > 0.0 {
                    (nx / len, ny / len, nz / len)
                } else {
                    (0.0, 0.0, 1.0)
                };

                // Pack to 0..255
                let px = ((nx * 0.5 + 0.5) * 255.0).round() as u8;
                let py = ((ny * 0.5 + 0.5) * 255.0).round() as u8;
                let pz = ((nz * 0.5 + 0.5) * 255.0).round() as u8;

                let o = ((y as usize) * self.width + (x as usize)) * 4;
                out[o] = px;
                out[o + 1] = py;
                out[o + 2] = pz;
                out[o + 3] = 255;
            }
        }

        self.normal_map = Some(Box::new(Texture {
            data: out,
            width: self.width,
            height: self.height,
            normal_map: None,
            material_map: None,
        }));
    }

    /// Returns the normal-map subtexture if present.
    pub fn normal_texture(&self) -> Option<&Texture> {
        self.normal_map.as_deref()
    }

    /// Samples the generated normal map at UV if present; otherwise returns a flat normal (0,0,1) encoded in RGBA8.
    pub fn sample_normal_rgba(
        &self,
        u: f32,
        v: f32,
        sample_mode: SampleMode,
        repeat_mode: RepeatMode,
    ) -> [u8; 4] {
        if let Some(norm) = self.normal_map.as_deref() {
            return norm.sample(u, v, sample_mode, repeat_mode);
        }
        [127, 127, 255, 255]
    }

    /// Sets the material map subtexture (R=roughness, G=metallic, B=opacity, A=unused)
    pub fn set_material_map(&mut self, tex: Texture) {
        self.material_map = Some(Box::new(tex));
    }

    /// Returns the material map subtexture if present.
    pub fn material_texture(&self) -> Option<&Texture> {
        self.material_map.as_deref()
    }

    /// Samples the material map at UV if present; returns (roughness, metallic, opacity) in 0..1.
    /// If no map is present, returns sensible defaults: (roughness=0.5, metallic=0.0, opacity=1.0).
    #[inline(always)]
    pub fn sample_material(
        &self,
        u: f32,
        v: f32,
        sample_mode: SampleMode,
        repeat_mode: RepeatMode,
    ) -> (f32, f32, f32) {
        if let Some(mat) = self.material_map.as_deref() {
            let rgba = mat.sample(u, v, sample_mode, repeat_mode);
            // R=roughness, G=metallic, B=opacity
            let r = rgba[0] as f32 / 255.0;
            let m = rgba[1] as f32 / 255.0;
            let o = rgba[2] as f32 / 255.0;
            (r, m, o)
        } else {
            (0.5, 0.0, 1.0)
        }
    }

    /// Convenience: write a single (roughness, metallic, opacity) triplet into the material map at (x,y),
    /// allocating a new material map if needed. Values are expected in 0..1 and are converted to RGBA8.
    pub fn set_material_pixel(
        &mut self,
        x: u32,
        y: u32,
        roughness: f32,
        metallic: f32,
        opacity: f32,
    ) {
        if self.material_map.is_none() {
            // allocate a sibling texture for the material map with same dimensions
            let data = vec![0u8; self.width * self.height * 4];
            self.material_map = Some(Box::new(Texture {
                data,
                width: self.width,
                height: self.height,
                normal_map: None,
                material_map: None,
            }));
        }
        if let Some(mat) = self.material_map.as_deref_mut() {
            let x = x.min((self.width - 1) as u32) as usize;
            let y = y.min((self.height - 1) as u32) as usize;
            let idx = (y * self.width + x) * 4;
            mat.data[idx] = (roughness * 255.0).round() as u8;
            mat.data[idx + 1] = (metallic * 255.0).round() as u8;
            mat.data[idx + 2] = (opacity * 255.0).round() as u8;
            mat.data[idx + 3] = 255;
        }
    }

    /// Applies a MaterialProfile across the texture and writes (roughness, metallic) into the material map.
    pub fn bake_material_profile(&mut self, profile: MaterialProfile, k: f32) {
        if self.material_map.is_none() {
            let data = vec![0u8; self.width * self.height * 4];
            self.material_map = Some(Box::new(Texture {
                data,
                width: self.width,
                height: self.height,
                normal_map: None,
                material_map: None,
            }));
        }

        // Iterate over pixels
        let w = self.width as usize;
        let h = self.height as usize;
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) * 4;
                // Base color → Vec3<f32>
                let color = Vec3::new(
                    self.data[idx] as f32 / 255.0,
                    self.data[idx + 1] as f32 / 255.0,
                    self.data[idx + 2] as f32 / 255.0,
                );

                // Read current base material (original) using direct access if available, else defaults
                let (base_r, base_m, base_o) = if let Some(mat) = self.material_map.as_deref() {
                    let m = &mat.data[idx..idx + 4];
                    (
                        m[0] as f32 / 255.0,
                        m[1] as f32 / 255.0,
                        m[2] as f32 / 255.0,
                    )
                } else {
                    (0.5, 0.0, 0.0)
                };

                // Target from profile at full effect
                let (target_m, target_r) = profile.evaluate_target(color);

                // Blend according to k so k=0 keeps base, k=1 goes fully to profile
                let r_final = lerp(base_r, target_r, k);
                let m_final = lerp(base_m, target_m, k);

                // Write back into material map
                self.set_material_pixel(x as u32, y as u32, r_final, m_final, base_o);
            }
        }
    }
}
