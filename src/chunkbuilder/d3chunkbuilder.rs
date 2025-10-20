use crate::{Assets, Batch3D, Chunk, ChunkBuilder, Map, PixelSource, Value};
use crate::{GeometrySource, LoopOp, ProfileLoop, RepeatMode, Sector};
use vek::{Vec2, Vec3};

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

            // Occlusion data
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut occl_bbox = bbox.clone();
                occl_bbox.expand(Vec2::new(0.1, 0.1));
                chunk.occluded_sectors.push((occl_bbox, occlusion));
            }

            // Try to get profile loops from sector/map; if available, run base + features; else fallback.
            if let Some((outer_loop, hole_loops)) = read_profile_loops(surface, sector, map) {
                let dbg = false;
                if dbg {
                    println!(
                        "[DBG] build surface={}, sector={}",
                        surface.sector_id, sector.id
                    );
                    dump_poly("outer_loop", &outer_loop.path);
                    for (i, h) in hole_loops.iter().enumerate() {
                        println!("[DBG]  hole[{}] op={:?}", i, h.op);
                        dump_poly(&format!("hole[{}]", i), &h.path);
                    }
                }
                let (_base_holes, feature_loops) = split_loops_for_base(&outer_loop, &hole_loops);

                // 1) BASE WALL from profile loops (outer with holes)
                let mut outer_path = outer_loop.path.clone();
                let mut holes_paths: Vec<Vec<vek::Vec2<f32>>> =
                    hole_loops.iter().map(|h| h.path.clone()).collect();

                if dbg {
                    let total_pts: usize =
                        outer_path.len() + holes_paths.iter().map(|h| h.len()).sum::<usize>();
                    println!(
                        "[DBG] earcut_with_holes: outer_pts={}, holes={}, total_pts={}",
                        outer_path.len(),
                        holes_paths.len(),
                        total_pts
                    );
                }
                if let Some((verts_uv, indices)) =
                    earcut_with_holes(&mut outer_path, &mut holes_paths)
                {
                    // Map UV -> world via surface
                    let world_vertices: Vec<[f32; 4]> = verts_uv
                        .iter()
                        .map(|uv| {
                            let p = surface.uv_to_world(vek::Vec2::new(uv[0], uv[1]));
                            [p.x, p.y, p.z, 1.0]
                        })
                        .collect();

                    if dbg {
                        println!(
                            "[DBG] earcut ok: verts_uv={}, tris={}",
                            verts_uv.len(),
                            indices.len()
                        );
                    }
                    let mut indices = indices; // make mutable copy from earcut
                    let desired_n = surface.plane.normal;
                    fix_winding(&world_vertices, &mut indices, desired_n);

                    if dbg {
                        if let Some((a, b, c)) = indices.get(0).cloned() {
                            let va = vek::Vec3::new(
                                world_vertices[a][0],
                                world_vertices[a][1],
                                world_vertices[a][2],
                            );
                            let vb = vek::Vec3::new(
                                world_vertices[b][0],
                                world_vertices[b][1],
                                world_vertices[b][2],
                            );
                            let vc = vek::Vec3::new(
                                world_vertices[c][0],
                                world_vertices[c][1],
                                world_vertices[c][2],
                            );
                            let n = (vb - va).cross(vc - va);
                            let nn = {
                                let l = n.magnitude();
                                if l > 1e-6 { n / l } else { n }
                            };
                            let dn = {
                                let d = surface.plane.normal;
                                let l = d.magnitude();
                                if l > 1e-6 { d / l } else { d }
                            };
                            println!(
                                "[DBG] base tri[0] normal=({:.3},{:.3},{:.3}) dot surfN={:.3}",
                                nn.x,
                                nn.y,
                                nn.z,
                                nn.dot(dn)
                            );
                        }
                    }

                    // --- UV build (same as before) ---
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
                        let tex_scale_x =
                            sector.properties.get_float_default("texture_scale_x", 1.0);
                        let tex_scale_y =
                            sector.properties.get_float_default("texture_scale_y", 1.0);
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
                                let mut batch = Batch3D::new(
                                    world_vertices.clone(),
                                    indices.clone(),
                                    uvs.clone(),
                                )
                                .repeat_mode(RepeatMode::RepeatXY)
                                .source(PixelSource::StaticTileIndex(texture_index))
                                .geometry_source(GeometrySource::Sector(sector.id));
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
                        let mut batch =
                            Batch3D::new(world_vertices.clone(), indices.clone(), uvs.clone())
                                .repeat_mode(RepeatMode::RepeatXY)
                                .geometry_source(GeometrySource::Sector(sector.id));
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

                    // --- Extrusion: thickness, back cap, side bands ---
                    if surface.extrusion.enabled && surface.extrusion.depth.abs() > 1e-6 {
                        let depth = surface.extrusion.depth;
                        let n = {
                            let nn = surface.plane.normal;
                            let l = nn.magnitude();
                            if l > 1e-6 {
                                nn / l
                            } else {
                                vek::Vec3::unit_y()
                            }
                        };

                        // 1) Back cap at z = depth (offset along normal)
                        {
                            let back_world_vertices: Vec<[f32; 4]> = verts_uv
                                .iter()
                                .map(|uv| {
                                    let p = surface.uv_to_world(vek::Vec2::new(uv[0], uv[1]))
                                        + n * depth;
                                    [p.x, p.y, p.z, 1.0]
                                })
                                .collect();

                            let mut back_indices = indices.clone();
                            // Faces should point opposite to front cap
                            fix_winding(
                                &back_world_vertices,
                                &mut back_indices,
                                -surface.plane.normal,
                            );

                            let back_uvs = uvs.clone();

                            let mut back_batch =
                                Batch3D::new(back_world_vertices, back_indices, back_uvs)
                                    .repeat_mode(RepeatMode::RepeatXY)
                                    .geometry_source(GeometrySource::Sector(sector.id));
                            if let Some(si) = shader_index {
                                back_batch.shader = Some(si);
                                if chunk.shaders_with_opacity[si] {
                                    chunk.batches3d_opacity.push(back_batch);
                                } else {
                                    chunk.batches3d.push(back_batch);
                                }
                            } else {
                                back_batch.source = PixelSource::Pixel([128, 128, 128, 255]);
                                chunk.batches3d.push(back_batch);
                            }
                        }

                        // Helper to push a side band (outer ring or through-hole tube)
                        let mut push_side_band = |loop_uv: &Vec<vek::Vec2<f32>>| {
                            let (ring_v, mut ring_i, ring_uv) = build_jamb(surface, loop_uv, depth);
                            fix_winding(&ring_v, &mut ring_i, surface.plane.normal);
                            let mut band_batch = Batch3D::new(ring_v, ring_i, ring_uv)
                                .repeat_mode(RepeatMode::RepeatXY)
                                .geometry_source(GeometrySource::Sector(sector.id));
                            if let Some(si) = shader_index {
                                band_batch.shader = Some(si);
                                if chunk.shaders_with_opacity[si] {
                                    chunk.batches3d_opacity.push(band_batch);
                                } else {
                                    chunk.batches3d.push(band_batch);
                                }
                            } else {
                                band_batch.source = PixelSource::Pixel([128, 128, 128, 255]);
                                chunk.batches3d.push(band_batch);
                            }
                        };

                        // 2) Outer perimeter side band
                        push_side_band(&outer_loop.path);

                        // 3) Through-hole tubes for holes that actually go through the thickness
                        let eps = 1e-5f32;
                        for h in &hole_loops {
                            let through = match h.op {
                                LoopOp::None => true,
                                LoopOp::Recess { depth: d } => d + eps >= depth.abs(),
                                LoopOp::Relief { .. } => false,
                            };
                            if through {
                                push_side_band(&h.path);
                            }
                        }
                    }
                }

                // 2) FEATURE LOOPS: build caps + jambs
                for fl in feature_loops {
                    match fl.op {
                        LoopOp::Relief { height } if height > 0.0 => {
                            if let Some((cap_v, cap_i, cap_uv)) =
                                build_cap(surface, &fl.path, height)
                            {
                                let mut cap_i = cap_i;
                                // Relief cap faces along +normal; recess cap faces along -normal
                                let desired_n = match fl.op {
                                    LoopOp::Relief { .. } => surface.plane.normal,
                                    LoopOp::Recess { .. } => -surface.plane.normal,
                                    _ => surface.plane.normal,
                                };
                                fix_winding(&cap_v, &mut cap_i, desired_n);

                                let mut batch = Batch3D::new(cap_v, cap_i, cap_uv)
                                    .repeat_mode(RepeatMode::RepeatXY)
                                    .geometry_source(GeometrySource::Sector(sector.id));
                                if let Some(si) = feature_shader_index(
                                    surface,
                                    map,
                                    sector,
                                    fl.origin_profile_sector,
                                    chunk,
                                ) {
                                    batch.shader = Some(si);
                                }
                                if let Some(Value::Source(pixelsource)) = feature_pixelsource(
                                    surface,
                                    map,
                                    sector,
                                    fl.origin_profile_sector,
                                    "relief_source",
                                ) {
                                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                        if let Some(tex) = assets.tile_index(&tile.id) {
                                            batch.source = PixelSource::StaticTileIndex(tex);
                                        }
                                    }
                                }
                                chunk.batches3d.push(batch);
                            }
                            let (ring_v, ring_i, ring_uv) = build_jamb(surface, &fl.path, height);
                            let mut ring_i = ring_i;
                            fix_winding(&ring_v, &mut ring_i, surface.plane.normal);

                            let mut batch = Batch3D::new(ring_v, ring_i, ring_uv)
                                .repeat_mode(RepeatMode::RepeatXY)
                                .geometry_source(GeometrySource::Sector(sector.id));
                            if let Some(si) = feature_shader_index(
                                surface,
                                map,
                                sector,
                                fl.origin_profile_sector,
                                chunk,
                            ) {
                                batch.shader = Some(si);
                            }
                            if let Some(Value::Source(pixelsource)) = feature_pixelsource(
                                surface,
                                map,
                                sector,
                                fl.origin_profile_sector,
                                "relief_jamb_source",
                            ) {
                                if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                    if let Some(tex) = assets.tile_index(&tile.id) {
                                        batch.source = PixelSource::StaticTileIndex(tex);
                                    }
                                }
                            }
                            chunk.batches3d.push(batch);
                        }
                        LoopOp::Recess { depth } if depth > 0.0 => {
                            if let Some((cap_v, cap_i, cap_uv)) =
                                build_cap(surface, &fl.path, -depth)
                            {
                                let mut cap_i = cap_i;
                                // Recess cap faces along -normal
                                let desired_n = -surface.plane.normal;
                                fix_winding(&cap_v, &mut cap_i, desired_n);

                                if dbg {
                                    println!(
                                        "[DBG] recess cap: verts={}, tris={}",
                                        cap_v.len(),
                                        cap_i.len()
                                    );
                                }

                                let mut batch = Batch3D::new(cap_v, cap_i, cap_uv)
                                    .repeat_mode(RepeatMode::RepeatXY)
                                    .geometry_source(GeometrySource::Sector(sector.id));
                                if let Some(si) = feature_shader_index(
                                    surface,
                                    map,
                                    sector,
                                    fl.origin_profile_sector,
                                    chunk,
                                ) {
                                    batch.shader = Some(si);
                                }
                                if let Some(Value::Source(pixelsource)) = feature_pixelsource(
                                    surface,
                                    map,
                                    sector,
                                    fl.origin_profile_sector,
                                    "recess_source",
                                ) {
                                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                        if let Some(tex) = assets.tile_index(&tile.id) {
                                            batch.source = PixelSource::StaticTileIndex(tex);
                                        }
                                    }
                                }
                                chunk.batches3d.push(batch);
                            }
                            let (ring_v, ring_i, ring_uv) = build_jamb(surface, &fl.path, -depth);
                            let mut ring_i = ring_i;
                            fix_winding(&ring_v, &mut ring_i, surface.plane.normal);
                            let mut batch = Batch3D::new(ring_v, ring_i, ring_uv)
                                .repeat_mode(RepeatMode::RepeatXY)
                                .geometry_source(GeometrySource::Sector(sector.id));
                            if let Some(si) = feature_shader_index(
                                surface,
                                map,
                                sector,
                                fl.origin_profile_sector,
                                chunk,
                            ) {
                                batch.shader = Some(si);
                            }
                            if let Some(Value::Source(pixelsource)) = feature_pixelsource(
                                surface,
                                map,
                                sector,
                                fl.origin_profile_sector,
                                "recess_jamb_source",
                            ) {
                                if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                    if let Some(tex) = assets.tile_index(&tile.id) {
                                        batch.source = PixelSource::StaticTileIndex(tex);
                                    }
                                }
                            }
                            chunk.batches3d.push(batch);
                        }
                        _ => {}
                    }
                }
            } else {
                // Fallback: no profile info; triangulate whole surface as-is
                if let Some((world_vertices, indices, verts_uv)) = surface.triangulate(sector, map)
                {
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
                        let tex_scale_x =
                            sector.properties.get_float_default("texture_scale_x", 1.0);
                        let tex_scale_y =
                            sector.properties.get_float_default("texture_scale_y", 1.0);
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
                                let mut batch = Batch3D::new(
                                    world_vertices.clone(),
                                    indices.clone(),
                                    uvs.clone(),
                                )
                                .repeat_mode(RepeatMode::RepeatXY)
                                .source(PixelSource::StaticTileIndex(texture_index))
                                .geometry_source(GeometrySource::Sector(sector.id));
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
                            .repeat_mode(RepeatMode::RepeatXY)
                            .geometry_source(GeometrySource::Sector(sector.id));
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
}

// --- Relief/recess pipeline helpers ---
/// Classify profile loops: all holes go into the base as holes; only Relief/Recess loops
/// also produce feature meshes (cap + jamb ring).
fn split_loops_for_base<'a>(
    _outer: &'a ProfileLoop,
    holes: &'a [ProfileLoop],
) -> (Vec<&'a ProfileLoop>, Vec<&'a ProfileLoop>) {
    let mut base_holes = Vec::new();
    let mut feature_loops = Vec::new();
    for h in holes {
        base_holes.push(h);
        match h.op {
            LoopOp::Relief { .. } | LoopOp::Recess { .. } => feature_loops.push(h),
            LoopOp::None => {}
        }
    }
    (base_holes, feature_loops)
}

/// Triangulate a simple polygon (no holes) in UV space using earcutr and return indices.
fn earcut_simple(loop_uv: &[vek::Vec2<f32>]) -> Option<Vec<(usize, usize, usize)>> {
    if loop_uv.len() < 3 {
        return None;
    }
    let flat: Vec<f64> = loop_uv
        .iter()
        .flat_map(|p| [p.x as f64, p.y as f64])
        .collect();
    let idx = earcutr::earcut(&flat, &[], 2).ok()?;
    let tris = idx.chunks_exact(3).map(|t| (t[2], t[1], t[0])).collect();
    Some(tris)
}

/// Build a cap for a relief/recess by triangulating the loop and offsetting along surface normal.
fn build_cap(
    surface: &crate::Surface,
    loop_uv: &[vek::Vec2<f32>],
    offset: f32, // +height (relief) or -depth (recess)
) -> Option<(Vec<[f32; 4]>, Vec<(usize, usize, usize)>, Vec<[f32; 2]>)> {
    let n = {
        let nn = surface.plane.normal;
        let len = nn.magnitude();
        if len > 1e-6 {
            nn / len
        } else {
            vek::Vec3::unit_y()
        }
    };
    let verts_world: Vec<[f32; 4]> = loop_uv
        .iter()
        .map(|uv| {
            let p = surface.uv_to_world(*uv) + n * offset;
            [p.x, p.y, p.z, 1.0]
        })
        .collect();
    let tris = earcut_simple(loop_uv)?;
    let uvs: Vec<[f32; 2]> = loop_uv.iter().map(|p| [p.x, p.y]).collect();
    Some((verts_world, tris, uvs))
}

/// Build the jamb (side ring) that connects the base plane loop to the displaced cap loop.
fn build_jamb(
    surface: &crate::Surface,
    loop_uv: &[vek::Vec2<f32>],
    offset: f32,
) -> (Vec<[f32; 4]>, Vec<(usize, usize, usize)>, Vec<[f32; 2]>) {
    let n = {
        let nn = surface.plane.normal;
        let len = nn.magnitude();
        if len > 1e-6 {
            nn / len
        } else {
            vek::Vec3::unit_y()
        }
    };
    let m = loop_uv.len();
    let mut verts: Vec<[f32; 4]> = Vec::with_capacity(m * 2);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(m * 2);

    // Base ring then displaced ring
    for uv in loop_uv {
        let p0 = surface.uv_to_world(*uv);
        verts.push([p0.x, p0.y, p0.z, 1.0]);
        uvs.push([0.0, 0.0]);
    }
    for uv in loop_uv {
        let p1 = surface.uv_to_world(*uv) + n * offset;
        verts.push([p1.x, p1.y, p1.z, 1.0]);
        uvs.push([0.0, 1.0]);
    }

    // Perimeter for U mapping
    let mut perim = 0.0f32;
    for i in 0..m {
        let a = loop_uv[i];
        let b = loop_uv[(i + 1) % m];
        perim += (b - a).magnitude();
    }

    let mut idx: Vec<(usize, usize, usize)> = Vec::with_capacity(m * 2);
    let mut cum = 0.0f32;
    for i in 0..m {
        let a = loop_uv[i];
        let b = loop_uv[(i + 1) % m];
        let seg = (b - a).magnitude();
        let u0 = if perim > 1e-6 { cum / perim } else { 0.0 };
        let u1 = if perim > 1e-6 {
            (cum + seg) / perim
        } else {
            1.0
        };

        let i0 = i;
        let i1 = (i + 1) % m;
        let j0 = m + i;
        let j1 = m + ((i + 1) % m);

        idx.push((i0, i1, j1));
        idx.push((i0, j1, j0));

        uvs[i0][0] = u0;
        uvs[j0][0] = u0;
        uvs[i1][0] = u1;
        uvs[j1][0] = u1;

        cum += seg;
    }

    (verts, idx, uvs)
}

/// Read profile loops (outer + holes) for a surface from the profile map, using profile sectors.
fn read_profile_loops(
    surface: &crate::Surface,
    _sector: &Sector,
    map: &Map,
) -> Option<(ProfileLoop, Vec<ProfileLoop>)> {
    // 1) OUTER from the host sector geometry (projected to UV)
    let outer_path = match project_sector_to_uv(surface, _sector, map) {
        Some(p) if p.len() >= 3 => p,
        _ => return None,
    };

    // Read outer-loop op from the host sector if present
    let outer_op_code = _sector.properties.get_int_default("profile_outer_op", 0);
    let outer_op = match outer_op_code {
        1 => LoopOp::Relief {
            height: _sector
                .properties
                .get_float_default("profile_outer_height", 0.0),
        },
        2 => LoopOp::Recess {
            depth: _sector
                .properties
                .get_float_default("profile_outer_depth", 0.0),
        },
        _ => LoopOp::None,
    };
    let outer = ProfileLoop {
        path: outer_path,
        op: outer_op,
        origin_profile_sector: None,
    };

    // 2) HOLES from the profile map for this surface
    let mut holes: Vec<ProfileLoop> = Vec::new();
    if let Some(profile_id) = surface.profile {
        if let Some(profile_map) = map.profiles.get(&profile_id) {
            for ps in profile_map.sectors.iter() {
                // Build UV path from the profile sector boundary (2D profile space).
                // Editor convention: -Y is up → flip Y here.
                let mut uv_path: Vec<vek::Vec2<f32>> = Vec::new();
                for &ld_id in ps.linedefs.iter() {
                    let ld = match profile_map.find_linedef(ld_id) {
                        Some(x) => x,
                        None => continue,
                    };
                    let v = match profile_map.get_vertex(ld.start_vertex) {
                        Some(x) => x,
                        None => continue,
                    };
                    let uv = vek::Vec2::new(v.x, -v.y);
                    if uv_path.last().map(|p| (p.x, p.y)) != Some((uv.x, uv.y)) {
                        uv_path.push(uv);
                    }
                }
                if uv_path.len() < 3 {
                    continue;
                }
                if (uv_path[0] - *uv_path.last().unwrap()).magnitude_squared() < 1e-8 {
                    uv_path.pop();
                }

                // Op comes from the profile sector itself
                let op_code = ps.properties.get_int_default("profile_op", 0);
                let op = match op_code {
                    1 => LoopOp::Relief {
                        height: ps.properties.get_float_default("profile_height", 0.0),
                    },
                    2 => LoopOp::Recess {
                        depth: ps.properties.get_float_default("profile_depth", 0.0),
                    },
                    _ => LoopOp::None,
                };
                holes.push(ProfileLoop {
                    path: uv_path,
                    op,
                    origin_profile_sector: Some(ps.id as u32),
                });
            }
        }
    }

    Some((outer, holes))
}

fn ensure_ccw(poly: &mut [vek::Vec2<f32>]) {
    if polygon_area(poly) < 0.0 {
        poly.reverse();
    }
}
fn ensure_cw(poly: &mut [vek::Vec2<f32>]) {
    if polygon_area(poly) > 0.0 {
        poly.reverse();
    }
}

/// Triangulate an outer polygon with holes in UV space using earcutr.
/// Returns (verts_uv, indices) where verts_uv = [outer..., hole0..., hole1..., ...]
fn earcut_with_holes(
    outer: &mut Vec<vek::Vec2<f32>>,
    holes: &mut [Vec<vek::Vec2<f32>>],
) -> Option<(Vec<[f32; 2]>, Vec<(usize, usize, usize)>)> {
    // Winding for earcut: outer CW, holes CCW (works with our flipped-Y editor space)
    ensure_cw(outer);
    for h in holes.iter_mut() {
        ensure_ccw(h);
    }

    // Flatten vertices: outer then each hole
    let mut verts_uv: Vec<[f32; 2]> = Vec::new();
    let mut holes_idx: Vec<usize> = Vec::new();

    for p in outer.iter() {
        verts_uv.push([p.x, p.y]);
    }
    let mut acc = outer.len();
    for h in holes.iter() {
        holes_idx.push(acc);
        acc += h.len();
        for p in h.iter() {
            verts_uv.push([p.x, p.y]);
        }
    }

    // Build f64 flat list
    let flattened: Vec<f64> = verts_uv
        .iter()
        .flat_map(|v| [v[0] as f64, v[1] as f64])
        .collect();

    // Run earcut
    let idx = earcutr::earcut(&flattened, &holes_idx, 2).ok()?;
    let indices: Vec<(usize, usize, usize)> =
        idx.chunks_exact(3).map(|c| (c[2], c[1], c[0])).collect();

    Some((verts_uv, indices))
}

fn fix_winding(
    world_vertices: &[[f32; 4]],
    indices: &mut Vec<(usize, usize, usize)>,
    desired_normal: vek::Vec3<f32>,
) {
    if indices.is_empty() {
        return;
    }
    // Average a few triangle normals (robust if the first is degenerate)
    let mut acc = vek::Vec3::zero();
    for (a, b, c) in indices.iter().take(8) {
        let va = vek::Vec3::new(
            world_vertices[*a][0],
            world_vertices[*a][1],
            world_vertices[*a][2],
        );
        let vb = vek::Vec3::new(
            world_vertices[*b][0],
            world_vertices[*b][1],
            world_vertices[*b][2],
        );
        let vc = vek::Vec3::new(
            world_vertices[*c][0],
            world_vertices[*c][1],
            world_vertices[*c][2],
        );
        acc += (vb - va).cross(vc - va);
    }
    let len: f32 = acc.magnitude();
    if len < 1e-8 {
        return;
    }
    let face_n: Vec3<f32> = acc / len;
    if face_n.dot(desired_normal) < 0.0 {
        for tri in indices.iter_mut() {
            core::mem::swap(&mut tri.1, &mut tri.2);
        }
    }
}

fn poly_winding(poly: &[vek::Vec2<f32>]) -> &'static str {
    if polygon_area(poly) > 0.0 {
        "CCW"
    } else {
        "CW"
    }
}

fn dump_poly(label: &str, poly: &[vek::Vec2<f32>]) {
    println!(
        "[DBG] {}: len={}, area={:.6}, winding={}",
        label,
        poly.len(),
        polygon_area(poly).abs(),
        poly_winding(poly)
    );
    for (i, p) in poly.iter().enumerate().take(12) {
        println!("    [{}] ({:.4}, {:.4})", i, p.x, p.y);
    }
    if poly.len() > 12 {
        println!("    ... ({} points total)", poly.len());
    }
}

// --- Profile geometry helpers ---
/// Project a sector boundary (start-vertex ordered) into a surface's UV plane.
fn project_sector_to_uv(
    surface: &crate::Surface,
    sector: &Sector,
    map: &Map,
) -> Option<Vec<vek::Vec2<f32>>> {
    let mut uv: Vec<vek::Vec2<f32>> = Vec::new();
    for &ld_id in sector.linedefs.iter() {
        let ld = map.find_linedef(ld_id)?;
        let v = map.get_vertex_3d(ld.start_vertex)?; // world xyz with Y up
        let p = vek::Vec3::new(v.x, v.y, v.z);
        let q = surface.world_to_uv(p);
        if uv.last().map(|w| (w.x, w.y)) != Some((q.x, q.y)) {
            uv.push(q);
        }
    }
    if uv.len() < 3 {
        return None;
    }
    // drop duplicate last==first
    if (uv[0] - *uv.last().unwrap()).magnitude_squared() < 1e-8 {
        uv.pop();
    }
    Some(uv)
}

fn polygon_area(poly: &[vek::Vec2<f32>]) -> f32 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut a2 = 0.0f32; // 2*A
    for i in 0..n {
        let p = poly[i];
        let q = poly[(i + 1) % n];
        a2 += p.x * q.y - q.x * p.y;
    }
    0.5 * a2
}

fn feature_pixelsource(
    surface: &crate::Surface,
    map: &Map,
    host_sector: &Sector,
    loop_origin: Option<u32>,
    key: &str,
) -> Option<Value> {
    // Prefer per-feature property on the originating profile sector
    if let (Some(profile_id), Some(origin_id)) = (surface.profile, loop_origin) {
        if let Some(profile_map) = map.profiles.get(&profile_id) {
            if let Some(ps) = profile_map.find_sector(origin_id) {
                if let Some(v) = ps.properties.get(key) {
                    return Some(v.clone());
                }
            }
        }
    }
    // Fallback to host sector property
    host_sector.properties.get(key).cloned()
}

fn feature_shader_index(
    surface: &crate::Surface,
    map: &Map,
    host_sector: &Sector,
    loop_origin: Option<u32>,
    chunk: &mut Chunk,
) -> Option<usize> {
    // Prefer per-feature shader on the originating profile sector
    if let (Some(profile_id), Some(origin_id)) = (surface.profile, loop_origin) {
        if let Some(profile_map) = map.profiles.get(&profile_id) {
            if let Some(ps) = profile_map.find_sector(origin_id) {
                if let Some(shader_id) = ps.shader {
                    if let Some(m) = map.shaders.get(&shader_id) {
                        if let Some(si) = chunk.add_shader(&m.build_shader()) {
                            return Some(si);
                        }
                    }
                }
            }
        }
    }
    // Fallback to host sector shader
    if let Some(shader_id) = host_sector.shader {
        if let Some(m) = map.shaders.get(&shader_id) {
            return chunk.add_shader(&m.build_shader());
        }
    }
    None
}
