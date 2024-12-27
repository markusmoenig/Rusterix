// use crate::PrimitiveMode::*;
use crate::SceneBuilder;
use crate::Texture;
use crate::{Batch, Map, Scene, Tile};
use theframework::prelude::*;
use vek::Vec2;

pub struct D3Builder {}

impl SceneBuilder for D3Builder {
    fn new() -> Self {
        Self {}
    }

    fn build(
        &self,
        map: &Map,
        tiles: &FxHashMap<Uuid, Tile>,
        atlas: Texture,
        _screen_size: Vec2<f32>,
    ) -> Scene {
        let mut scene = Scene::empty();
        // let atlas_size = atlas.width as f32;

        let mut textures = vec![atlas];

        let atlas_batch = Batch::emptyd3();

        // Repeated tile textures have their own batches
        let mut repeated_batches: Vec<Batch<[f32; 4]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        // Create sectors
        for sector in &map.sectors {
            if let Some((vertices, indices)) = sector.generate_geometry(map) {
                // Generate floor geometry
                if let Some(floor_texture_id) = &sector.floor_texture {
                    if let Some(tile) = tiles.get(floor_texture_id) {
                        let floor_vertices = vertices
                            .iter()
                            .map(|&v| [v[0], sector.floor_height, v[1], 1.0])
                            .collect();

                        let floor_uvs = vertices.iter().map(|&v| [v[0], v[1]]).collect();

                        if let Some(offset) = repeated_offsets.get(&tile.id) {
                            repeated_batches[*offset].add(floor_vertices, indices, floor_uvs);
                        } else {
                            let texture_index = textures.len();

                            let mut batch = Batch::emptyd3()
                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                .sample_mode(crate::SampleMode::Nearest)
                                .texture_index(texture_index);

                            batch.add(floor_vertices, indices, floor_uvs);

                            textures.push(tile.textures[0].clone());
                            repeated_offsets.insert(tile.id, repeated_batches.len());
                            repeated_batches.push(batch);
                        }
                    }
                }

                /*
                // Generate ceiling geometry
                if let Some(ceiling_texture_id) = &sector.ceiling_texture {
                    //if let Some(el) = atlas_elements.get(ceiling_texture_id) {
                    let ceiling_vertices = floor_geo
                        .0
                        .iter()
                        .map(|&v| vec3f(v[0], sector.ceiling_height, v[1]))
                        .collect::<Vec<Vec3f>>();

                    /*
                    let ceiling_uvs = floor_geo
                        .0
                        .iter()
                        .map(|&v| {
                            let uv = vec2f(
                                el[0].x as f32
                                    + ((v[0] - bbox.0.x) / (bbox.1.x - bbox.0.x) * el[0].z as f32),
                                el[0].y as f32
                                    + ((v[1] - bbox.0.y) / (bbox.1.y - bbox.0.y) * el[0].w as f32),
                            );
                            uv / atlas_size
                        })
                        .collect::<Vec<Vec2f>>();*/
                    let ceiling_uvs = floor_geo
                        .0
                        .iter()
                        .map(|&v| vec2f(v[0], v[1]))
                        .collect::<Vec<Vec2f>>();

                    // let geometry =
                    //     Geometry::new(ceiling_vertices, floor_geo.1.clone(), ceiling_uvs);
                    // geometry_map.add(*ceiling_texture_id, geometry);
                    //}
                }*/

                // Generate wall geometry
                for &linedef_id in &sector.linedefs {
                    if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                        if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                            if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                                if let Some(wall_texture_id) = &linedef.texture {
                                    Self::add_wall(
                                        &start_vertex.as_vec2(),
                                        &end_vertex.as_vec2(),
                                        linedef.wall_height,
                                        wall_texture_id,
                                        tiles,
                                        &mut repeated_offsets,
                                        &mut repeated_batches,
                                        &mut textures,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add standalone walls
        for linedef in &map.linedefs {
            if linedef.front_sector.is_none() && linedef.back_sector.is_none() {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        if let Some(wall_texture_id) = &linedef.texture {
                            Self::add_wall(
                                &start_vertex.as_vec2(),
                                &end_vertex.as_vec2(),
                                linedef.wall_height,
                                wall_texture_id,
                                tiles,
                                &mut repeated_offsets,
                                &mut repeated_batches,
                                &mut textures,
                            );
                        }
                    }
                }
            }
        }

        // Add Sky
        if let Some(sky_texture_id) = map.sky_texture {
            Self::add_sky(
                &sky_texture_id,
                tiles,
                &mut repeated_offsets,
                &mut repeated_batches,
                &mut textures,
            );
        }

        // ---

        let mut batches = repeated_batches;
        batches.extend(vec![atlas_batch]);

        scene.d3_static = batches;
        scene.textures = textures;
        scene
    }
}

trait D3BuilderUtils {
    #[allow(clippy::too_many_arguments)]
    fn add_wall(
        start_vertex: &Vec2<f32>,
        end_vertex: &Vec2<f32>,
        wall_height: f32,
        wall_texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Texture>,
    );

    fn add_sky(
        texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Texture>,
    );
}

impl D3BuilderUtils for D3Builder {
    /// Adds a wall to the appropriate batch
    fn add_wall(
        start_vertex: &Vec2<f32>,
        end_vertex: &Vec2<f32>,
        wall_height: f32,
        wall_texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Texture>,
    ) {
        let wall_vertices = vec![
            [start_vertex.x, 0.0, start_vertex.y, 1.0],
            [start_vertex.x, wall_height, start_vertex.y, 1.0],
            [end_vertex.x, wall_height, end_vertex.y, 1.0],
            [end_vertex.x, 0.0, end_vertex.y, 1.0],
        ];

        if let Some(tile) = tiles.get(wall_texture_id) {
            let wall_uvs =
                if (end_vertex.x - start_vertex.x).abs() > (end_vertex.y - start_vertex.y).abs() {
                    // Wall is mostly aligned along the X-axis
                    vec![
                        [start_vertex.x, wall_height],
                        [start_vertex.x, 0.0],
                        [end_vertex.x, 0.0],
                        [end_vertex.x, wall_height],
                    ]
                } else {
                    // Wall is mostly aligned along the Z-axis
                    vec![
                        [start_vertex.y, wall_height],
                        [start_vertex.y, 0.0],
                        [end_vertex.y, 0.0],
                        [end_vertex.y, wall_height],
                    ]
                };

            let wall_indices = vec![(0, 1, 2), (0, 2, 3)];

            if let Some(offset) = repeated_offsets.get(&tile.id) {
                repeated_batches[*offset].add(wall_vertices, wall_indices, wall_uvs);
            } else {
                let texture_index = textures.len();

                let mut batch = Batch::emptyd3()
                    .repeat_mode(crate::RepeatMode::RepeatXY)
                    .cull_mode(crate::CullMode::Off)
                    .sample_mode(crate::SampleMode::Nearest)
                    .texture_index(texture_index);

                batch.add(wall_vertices, wall_indices, wall_uvs);

                textures.push(tile.textures[0].clone());
                repeated_offsets.insert(tile.id, repeated_batches.len());
                repeated_batches.push(batch);
            }
        }
    }

    /// Adds a skybox or skymap
    fn add_sky(
        texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Texture>,
    ) {
        // Define sky vertices
        let sky_vertices = vec![
            [-1000.0, 10.0, -1000.0, 1.0],
            [1000.0, 10.0, -1000.0, 1.0],
            [1000.0, 10.0, 1000.0, 1.0],
            [-1000.0, 10.0, 1000.0, 1.0],
        ];

        // Define UV coordinates for the sky texture
        let sky_uvs = vec![[0.0, 15.0], [15.0, 15.0], [15.0, 0.0], [0.0, 0.0]];

        // Define indices for rendering the quad
        let sky_indices = vec![(0, 1, 2), (0, 2, 3)];

        if let Some(tile) = tiles.get(texture_id) {
            if let Some(offset) = repeated_offsets.get(&tile.id) {
                // Add to an existing batch if the texture is already used
                repeated_batches[*offset].add(sky_vertices, sky_indices, sky_uvs);
            } else {
                // Create a new batch for the sky texture
                let texture_index = textures.len();

                let mut batch = Batch::emptyd3()
                    .repeat_mode(crate::RepeatMode::RepeatXY)
                    .cull_mode(crate::CullMode::Off)
                    .sample_mode(crate::SampleMode::Nearest)
                    .texture_index(texture_index);

                batch.add(sky_vertices, sky_indices, sky_uvs);

                textures.push(tile.textures[0].clone());
                repeated_offsets.insert(tile.id, repeated_batches.len());
                repeated_batches.push(batch);
            }
        }
    }
}
