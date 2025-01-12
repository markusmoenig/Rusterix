use rusterix::prelude::*;
use std::path::Path;
use vek::{Vec2, Vec3};

use criterion::{criterion_group, criterion_main, Criterion};

fn rasterize_map(c: &mut Criterion) {
    let mut camera: Box<dyn D3Camera> = Box::new(D3FirstPCamera::new());
    let mut scene = Scene::default();

    let mut assets = Assets::default();
    assets.collect_from_directory("minigame".into());
    let _ = assets.compile_source_map("world".into());

    if let Some(map) = assets.get_map("world") {
        let builder = D3Builder::new();
        scene = builder.build(
            map,
            &assets.tiles,
            Texture::from_color(BLACK),
            Vec2::zero(), // Only needed for 2D builders
            &camera.id(),
        );
    }

    // if let Ok(meta) = mapscript.transform(None, None, None) {
    //     // Build the 3D scene from the map meta data
    // }

    // Create an entity with a default position / orientation.
    let entity = rusterix::Entity {
        position: Vec3::new(6.0600824, 1.0, 4.5524735),
        orientation: Vec2::new(0.03489969, 0.99939084),
        ..Default::default()
    };

    // Add logo on top of the scene
    scene.d2 =
        vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0).texture_index(scene.textures.len())];
    scene
        .textures
        .push(Tile::from_texture(Texture::from_image(Path::new(
            "images/logo.png",
        ))));

    let width = 2000_usize;
    let height = 2000_usize;
    let mut pixels: Vec<u8> = vec![0; width * height * 4];

    entity.apply_to_camera(&mut camera);

    c.bench_function("rasterize_map", |b| {
        b.iter(|| {
            // Set it up
            Rasterizer::setup(
                None,
                camera.view_matrix(),
                camera.projection_matrix(width as f32, height as f32),
            )
            .rasterize(&mut scene, &mut pixels[..], width, height, 30);
        })
    });
}

criterion_group!(benches, rasterize_map);
criterion_main!(benches);
