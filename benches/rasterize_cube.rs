use rusterix::prelude::*;
use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};

fn rasterize_cube(c: &mut Criterion) {
    let mut scene = Scene::from_static(
        vec![Batch2D::from_rectangle(0.0, 0.0, 200.0, 200.0)],
        vec![Batch3D::from_box(-0.5, -0.5, -0.5, 1.0, 1.0, 1.0).cull_mode(CullMode::Off)],
    )
    .background(Box::new(VGrayGradientShader::new()))
    .textures(vec![Tile::from_texture(Texture::from_image(Path::new(
        "images/logo.png",
    )))]);

    let width = 2000_usize;
    let height = 2000_usize;
    let mut pixels: Vec<u8> = vec![0; width * height * 4];

    let camera = D3OrbitCamera::new();

    c.bench_function("rasterize_cube", |b| {
        b.iter(|| {
            // Set it up
            Rasterizer::setup(
                None,
                camera.view_matrix(),
                camera.projection_matrix(width as f32, height as f32),
            )
            .rasterize(&mut scene, &mut pixels[..], width, height, 40);
        })
    });
}

criterion_group!(benches, rasterize_cube);
criterion_main!(benches);
