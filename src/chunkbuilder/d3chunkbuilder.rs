use crate::{Assets, Batch3D, Chunk, ChunkBuilder, Map, PixelSource, Value};
use vek::Vec2;

pub struct D3ChunkBuilder {}

impl Clone for D3ChunkBuilder {
    fn clone(&self) -> Self {
        D3ChunkBuilder {}
    }
}

impl ChunkBuilder for D3ChunkBuilder {
    fn new() -> Self {
        Self {}
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder> {
        Box::new(self.clone())
    }

    fn build(&mut self, map: &Map, assets: &Assets, chunk: &mut Chunk) {
        // For each surface in the map
        for surface in map.surfaces.values() {
            let Some(sector) = map.find_sector(surface.sector_id) else {
                continue;
            };

            let bbox = sector.bounding_box(map);
            // Cull with the sector bbox: only use intersection
            if !bbox.intersects(&chunk.bbox) || !chunk.bbox.contains(bbox.center()) {
                continue;
            }
            // Collect occluded sectors and store them in the chunk
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut occl_bbox = bbox.clone();
                occl_bbox.expand(Vec2::new(0.1, 0.1));
                // chunk.occluded_sectors.push((occl_bbox, occlusion));
            }

            // Triangulate this surface in its own UV plane and map to world
            if let Some((world_vertices, indices, verts_uv)) = surface.triangulate(sector, map) {
                // Build UVs with tile_mode and texture_scale_x/y from the sector
                let tile_mode = sector.properties.get_int_default("tile_mode", 1);
                let mut minx = f32::INFINITY;
                let mut miny = f32::INFINITY;
                let mut maxx = f32::NEG_INFINITY;
                let mut maxy = f32::NEG_INFINITY;
                for v in &verts_uv {
                    minx = minx.min(v[0]);
                    maxx = maxx.max(v[0]);
                    miny = miny.min(v[1]);
                    maxy = maxy.max(v[1]);
                }
                let sx = (maxx - minx).max(1e-6);
                let sy = (maxy - miny).max(1e-6);
                let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(verts_uv.len());
                if tile_mode == 0 {
                    for v in &verts_uv {
                        uvs.push([(v[0] - minx) / sx, (v[1] - miny) / sy]);
                    }
                } else {
                    let tex_scale_x = sector.properties.get_float_default("texture_scale_x", 1.0);
                    let tex_scale_y = sector.properties.get_float_default("texture_scale_y", 1.0);
                    for v in &verts_uv {
                        uvs.push([(v[0] - minx) / tex_scale_x, (v[1] - miny) / tex_scale_y]);
                    }
                }

                let shader_index = sector
                    .shader
                    .and_then(|shader_id| {
                        map.shaders
                            .get(&shader_id)
                            .map(|m| chunk.add_shader(&m.build_shader()))
                    })
                    .flatten();
                let mut pushed = false;
                if let Some(Value::Source(pixelsource)) = sector.properties.get("source") {
                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                        if let Some(texture_index) = assets.tile_index(&tile.id) {
                            let mut batch =
                                Batch3D::new(world_vertices.clone(), indices.clone(), uvs.clone())
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .source(PixelSource::StaticTileIndex(texture_index))
                                    .geometry_source(crate::GeometrySource::Sector(sector.id));
                            if let Some(si) = shader_index {
                                batch.shader = Some(si);
                                if chunk.shaders_with_opacity[si] {
                                    chunk.batches3d_opacity.push(batch);
                                } else {
                                    chunk.batches3d.push(batch);
                                }
                            } else {
                                chunk.batches3d.push(batch);
                            }
                            pushed = true;
                        }
                    }
                }
                if !pushed {
                    let mut batch = Batch3D::new(world_vertices, indices, uvs)
                        .repeat_mode(crate::RepeatMode::RepeatXY)
                        .geometry_source(crate::GeometrySource::Sector(sector.id));
                    if let Some(si) = shader_index {
                        batch.shader = Some(si);
                        if chunk.shaders_with_opacity[si] {
                            chunk.batches3d_opacity.push(batch);
                        } else {
                            chunk.batches3d.push(batch);
                        }
                    } else {
                        batch.source = PixelSource::Pixel([128, 128, 128, 255]);
                        chunk.batches3d.push(batch);
                    }
                }
            }
            // Profile geometry is skipped for now
        }
    }

    /*
    fn build(&mut self, map: &Map, assets: &Assets, chunk: &mut Chunk) {
        // Create sectors
        for sector in &map.sectors {
            let bbox = sector.bounding_box(map);

            // Collect occluded sectors and store them in the chunk
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut occl_bbox = bbox.clone();
                occl_bbox.expand(Vec2::new(0.1, 0.1));
                chunk.occluded_sectors.push((occl_bbox, occlusion));
            }

            if bbox.intersects(&chunk.bbox) && chunk.bbox.contains(bbox.center()) {
                let mut add_it = true;

                // Special cases from the Rect tool
                let mut add_it_as_box = false;
                let mut add_it_as_floor = false;

                // Make sure we add Rect sectors with a rendering mode of "Box" as a box
                if sector.layer.is_some() {
                    let render_mode = sector.properties.get_int_default("rect_rendering", 0);
                    match render_mode {
                        0 => add_it = false,
                        1 => add_it_as_box = true,
                        2 => add_it_as_floor = true,
                        _ => {}
                    }
                }

                if add_it {
                    if let Some((vertices, indices)) = sector.generate_geometry(map) {
                        let shader_index = chunk.add_shader(&sector.module.build_shader());

                        let sector_elevation =
                            sector.properties.get_float_default("floor_height", 0.0);

                        // Generate floor geometry
                        if !add_it_as_box {
                            let mut processed = false;

                            let floor_vertices: Vec<[f32; 4]> = vertices
                                .iter()
                                .map(|&v| {
                                    [
                                        v[0],
                                        sector_elevation + if add_it_as_floor { 0.2 } else { 0.0 },
                                        v[1],
                                        1.0,
                                    ]
                                })
                                .collect();

                            // Build floor UVs with a switch between area-scaled and repeat mode
                            // tile_mode: 0 => scale to area [0..1]; 1 (default) => repeat using texture_scale
                            let tile_mode = sector.properties.get_int_default("tile_mode", 1);

                            // Compute local bbox of the floor vertices (in map space of this sector geometry)
                            let mut minx = f32::INFINITY;
                            let mut miny = f32::INFINITY;
                            let mut maxx = f32::NEG_INFINITY;
                            let mut maxy = f32::NEG_INFINITY;
                            for &v in &vertices {
                                minx = minx.min(v[0]);
                                maxx = maxx.max(v[0]);
                                miny = miny.min(v[1]);
                                maxy = maxy.max(v[1]);
                            }
                            let sx = (maxx - minx).max(1e-6);
                            let sy = (maxy - miny).max(1e-6);

                            let mut floor_uvs: Vec<[f32; 2]> = Vec::with_capacity(vertices.len());
                            if tile_mode == 0 {
                                // Normalize to [0..1] over the local area
                                for &v in &vertices {
                                    floor_uvs.push([(v[0] - minx) / sx, (v[1] - miny) / sy]);
                                }
                            } else {
                                // Repeat mode with per-axis texture scales
                                let tex_scale_x =
                                    sector.properties.get_float_default("texture_scale_x", 1.0);
                                let tex_scale_y =
                                    sector.properties.get_float_default("texture_scale_y", 1.0);
                                for &v in &vertices {
                                    floor_uvs.push([
                                        (v[0] - minx) / tex_scale_x,
                                        (v[1] - miny) / tex_scale_y,
                                    ]);
                                }
                            }

                            if let Some(Value::Source(pixelsource)) =
                                sector.properties.get("floor_source")
                            {
                                if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                    if let Some(texture_index) = assets.tile_index(&tile.id) {
                                        let mut batch = Batch3D::new(
                                            floor_vertices.clone(),
                                            indices.clone(),
                                            floor_uvs.clone(),
                                        )
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .source(PixelSource::StaticTileIndex(texture_index))
                                        .geometry_source(crate::GeometrySource::Sector(sector.id));
                                        batch.shader = shader_index;
                                        chunk.batches3d.push(batch);
                                        processed = true;
                                    }
                                }
                            }

                            if let Some(shader_index) = shader_index
                                && !processed
                            {
                                let batch =
                                    Batch3D::new(floor_vertices, indices.clone(), floor_uvs)
                                        .shader(shader_index)
                                        .geometry_source(crate::GeometrySource::Sector(sector.id));
                                chunk.batches3d.push(batch);
                            }
                        }

                        // Generate ceiling geometry

                        let create_ceiling = true;
                        // if camera_id == "iso"
                        //     && sector.properties.get_int_default("ceiling_in_iso", 0) == 1
                        // {
                        //     create_ceiling = false;
                        // }

                        if create_ceiling || add_it_as_box {
                            let material: Option<Material> =
                                crate::scenebuilder::get_material_from_geo_graph(
                                    &sector.properties,
                                    3,
                                    map,
                                );

                            let source = if add_it_as_box {
                                sector.properties.get("floor_source")
                            } else {
                                sector.properties.get("ceiling_source")
                            };

                            if let Some(Value::Source(pixelsource)) = &source {
                                if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                    if let Some(texture_index) = assets.tile_index(&tile.id) {
                                        let ceiling_vertices = vertices
                                            .iter()
                                            .map(|&v| {
                                                [
                                                    v[0],
                                                    sector
                                                        .properties
                                                        .get_float_default("ceiling_height", 0.0),
                                                    v[1],
                                                    1.0,
                                                ]
                                            })
                                            .collect();

                                        // Build ceiling UVs with a switch between area-scaled and repeat mode
                                        // tile_mode: 0 => scale to area [0..1]; 1 (default) => repeat using texture_scale
                                        let tile_mode =
                                            sector.properties.get_int_default("tile_mode", 1);

                                        // Compute local bbox of the ceiling vertices (map space of this sector geometry)
                                        let mut minx = f32::INFINITY;
                                        let mut miny = f32::INFINITY;
                                        let mut maxx = f32::NEG_INFINITY;
                                        let mut maxy = f32::NEG_INFINITY;
                                        for &v in &vertices {
                                            minx = minx.min(v[0]);
                                            maxx = maxx.max(v[0]);
                                            miny = miny.min(v[1]);
                                            maxy = maxy.max(v[1]);
                                        }
                                        let sx = (maxx - minx).max(1e-6);
                                        let sy = (maxy - miny).max(1e-6);

                                        let mut ceiling_uvs: Vec<[f32; 2]> =
                                            Vec::with_capacity(vertices.len());
                                        if tile_mode == 0 {
                                            // Normalize to [0..1] over the local area
                                            for &v in &vertices {
                                                ceiling_uvs
                                                    .push([(v[0] - minx) / sx, (v[1] - miny) / sy]);
                                            }
                                        } else {
                                            // Repeat mode with per-axis texture scales (defaults 1.0)
                                            let tex_scale_x = sector
                                                .properties
                                                .get_float_default("texture_scale_x", 1.0);
                                            let tex_scale_y = sector
                                                .properties
                                                .get_float_default("texture_scale_y", 1.0);
                                            for &v in &vertices {
                                                ceiling_uvs.push([
                                                    (v[0] - minx) / tex_scale_x,
                                                    (v[1] - miny) / tex_scale_y,
                                                ]);
                                            }
                                        }
                                        // let ceiling_indices =
                                        //     indices.iter().map(|&v| (v.2, v.1, v.0)).collect();

                                        let mut batch = Batch3D::new(
                                            ceiling_vertices,
                                            indices.clone(),
                                            ceiling_uvs,
                                        )
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .source(PixelSource::StaticTileIndex(texture_index));
                                        batch.material = material;
                                        chunk.batches3d.push(batch);

                                        /*
                                        if material.is_some() {
                                            let texture_index = textures.len();
                                            let mut batch = Batch::emptyd3()
                                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                                .texture_index(texture_index);
                                            batch.material = material;
                                            batch.add(ceiling_vertices, indices.clone(), ceiling_uvs);

                                            textures.push(tile.clone());
                                            repeated_offsets.insert(tile.id, repeated_batches.len());
                                            repeated_batches.push(batch);
                                        } else if let Some(offset) = repeated_offsets.get(&tile.id) {
                                            repeated_batches[*offset].add(
                                                ceiling_vertices,
                                                indices,
                                                ceiling_uvs,
                                            );
                                        } else {
                                            let texture_index = textures.len();

                                            let mut batch = Batch::emptyd3()
                                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                                .sample_mode(sample_mode)
                                                .texture_index(texture_index);

                                            batch.add(ceiling_vertices, indices, ceiling_uvs);

                                            textures.push(tile.clone());
                                            repeated_offsets.insert(tile.id, repeated_batches.len());
                                            repeated_batches.push(batch);
                                        }
                                        */
                                    }
                                }
                            }
                        }

                        // Generate wall geometry
                        if !add_it_as_floor {
                            for &linedef_id in &sector.linedefs {
                                if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                                    if !linedef.profile.vertices.is_empty() {
                                        // Profile Wall
                                        build_profile_wall(map, assets, chunk, linedef);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        /*
        // Add standalone walls
        for linedef in &map.linedefs {
            let bbox = linedef.bounding_box(map);
            if bbox.intersects(&chunk.bbox)
                && chunk.bbox.contains(bbox.center())
                && linedef.front_sector.is_none()
                && linedef.back_sector.is_none()
            {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        let repeat_sources =
                            linedef.properties.get_int_default("source_repeat", 0) == 0;
                        add_wall(
                            0.0,
                            &start_vertex.as_vec2(),
                            &end_vertex.as_vec2(),
                            linedef.properties.get_float_default("wall_height", 0.0),
                            linedef
                                .properties
                                .get("row1_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row2_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row3_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row4_source")
                                .and_then(|v| v.to_source()),
                            repeat_sources,
                            assets,
                            chunk,
                        );
                    }
                }
            }
        }*/
    }*/
}

/*
fn build_profile_wall(map: &Map, assets: &Assets, chunk: &mut Chunk, linedef: &Linedef) {
    if let (Some(start_vertex), Some(end_vertex)) = (
        map.find_vertex(linedef.start_vertex),
        map.find_vertex(linedef.end_vertex),
    ) {
        let start = start_vertex.as_vec2();
        let end = end_vertex.as_vec2();
        let delta = end - start;
        let len = delta.magnitude();
        if len <= 1e-6 {
            return;
        }
        let dir = delta / len; // unit direction along the wall in XZ plane

        // Nudge geometry slightly toward the front sector to avoid corner z-fighting/overlap
        let inward_normal = Vec2::new(-dir.y, dir.x); // left side of the edge is the front
        let default_eps = 0.001_f32;
        let eps = linedef
            .properties
            .get_float("profile_wall_epsilon")
            .unwrap_or(default_eps);
        // Positive moves toward front; negative toward back-only walls
        let offset2 = if linedef.front_sector.is_some() {
            inward_normal * eps
        } else if linedef.back_sector.is_some() {
            inward_normal * -eps
        } else {
            Vec2::new(0.0, 0.0)
        };

        // Base elevation from the front sector if present, otherwise 0.0
        let base_elevation = if let Some(front_id) = linedef.front_sector {
            if let Some(front) = map.sectors.get(front_id as usize) {
                front.properties.get_float_default("floor_height", 0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let profile = &linedef.profile;
        // Derive left/right anchors from profile vertex IDs; fallback to min/max if IDs missing
        let mut left_x = f32::INFINITY;
        let mut right_x = f32::NEG_INFINITY;
        for v in &profile.vertices {
            if let Some(id) = v.properties.get_int("profile_id") {
                match id {
                    1 | 2 => {
                        left_x = left_x.min(v.x);
                    } // left side
                    0 | 3 => {
                        right_x = right_x.max(v.x);
                    } // right side
                    _ => {}
                }
            }
        }
        if !left_x.is_finite() || !right_x.is_finite() {
            // Fallback: compute from all vertices
            left_x = f32::INFINITY;
            right_x = f32::NEG_INFINITY;
            for v in &profile.vertices {
                left_x = left_x.min(v.x);
                right_x = right_x.max(v.x);
            }
        }
        // Guard against degenerate width
        let denom = (right_x - left_x).max(1e-6);
        let sectors = profile.sorted_sectors_by_area();

        for sector in sectors {
            // Triangulate the 2D profile sector in its own map (profile)
            if let Some((pverts, pindices)) = sector.generate_geometry(profile) {
                // Optional shader/material per profile sector
                let shader_index = chunk.add_shader(&sector.module.build_shader());

                // Map 2D profile vertices to 3D world space along the wall plane
                // profile (x,y) -> world [x,z] = start + dir * mapped_x, world y = base_elevation + y
                let world_vertices: Vec<[f32; 4]> = pverts
                    .iter()
                    .map(|&v| {
                        let x = v[0];
                        let y = v[1];
                        // Normalize profile x relative to anchors and clamp into [0..len]
                        let mut t = (x - left_x) / denom; // 0..1 across the drawn profile
                        if t < 0.0 {
                            t = 0.0;
                        } else if t > 1.0 {
                            t = 1.0;
                        }
                        let along = t * len;
                        let pos2 = start + dir * along + offset2; // XZ plane mapping with slight inward offset
                        // Profile Y grows downward (0 at bottom, -1, -2 ...). Build upward in world by negating Y.
                        [pos2.x, base_elevation - y, pos2.y, 1.0]
                    })
                    .collect();

                // Build UVs with a switch between area-scaled and repeat mode
                // tile_mode: 0 => scale to area [0..1]; 1 (default) => repeat using texture_scale
                let tile_mode = sector.properties.get_int_default("tile_mode", 1);

                // Compute local bbox of the profile vertices (in profile space)
                let mut minx = f32::INFINITY;
                let mut miny = f32::INFINITY;
                let mut maxx = f32::NEG_INFINITY;
                let mut maxy = f32::NEG_INFINITY;
                for &v in &pverts {
                    minx = minx.min(v[0]);
                    maxx = maxx.max(v[0]);
                    miny = miny.min(v[1]);
                    maxy = maxy.max(v[1]);
                }
                let sx = (maxx - minx).max(1e-6);
                let sy = (maxy - miny).max(1e-6);

                let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(pverts.len());
                if tile_mode == 0 {
                    // Scale UVs to the area (0..1) based on bbox in profile space
                    for &v in &pverts {
                        uvs.push([(v[0] - minx) / sx, (v[1] - miny) / sy]);
                    }
                } else {
                    // Repeat mode: allow per-axis texture scale; default 1.0 means 1 unit = 1 UV
                    let tex_scale_x = sector.properties.get_float_default("texture_scale_x", 1.0);
                    let tex_scale_y = sector.properties.get_float_default("texture_scale_y", 1.0);
                    for &v in &pverts {
                        uvs.push([(v[0] - minx) / tex_scale_x, (v[1] - miny) / tex_scale_y]);
                    }
                }

                // Try a tile/source from the profile sector; fall back to shader-only
                let mut pushed = false;
                if let Some(Value::Source(pixelsource)) = sector.properties.get("floor_source") {
                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                        if let Some(texture_index) = assets.tile_index(&tile.id) {
                            let mut batch =
                                Batch3D::new(world_vertices.clone(), pindices.clone(), uvs.clone())
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .source(PixelSource::StaticTileIndex(texture_index))
                                    .profile_id(linedef.id)
                                    .geometry_source(crate::GeometrySource::Sector(sector.id));
                            if let Some(si) = shader_index {
                                batch.shader = Some(si);
                                if chunk.shaders_with_opacity[si] {
                                    chunk.batches3d_opacity.push(batch);
                                } else {
                                    chunk.batches3d.push(batch);
                                }
                            } else {
                                chunk.batches3d.push(batch);
                            }
                            pushed = true;
                        }
                    }
                }

                if !pushed {
                    let mut batch = Batch3D::new(world_vertices, pindices, uvs)
                        .repeat_mode(crate::RepeatMode::RepeatXY)
                        .profile_id(linedef.id)
                        .geometry_source(crate::GeometrySource::Sector(sector.id));
                    if let Some(si) = shader_index {
                        batch.shader = Some(si);
                        if chunk.shaders_with_opacity[si] {
                            chunk.batches3d_opacity.push(batch);
                        } else {
                            chunk.batches3d.push(batch);
                        }
                    } else {
                        batch.source = PixelSource::Pixel([128, 128, 128, 255]);
                        chunk.batches3d.push(batch);
                    }
                }
            }
        }
    }
}
*/
