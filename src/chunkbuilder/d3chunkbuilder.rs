use crate::{Assets, Batch3D, Chunk, ChunkBuilder, Map, PixelSource, Value};
use crate::{GeometrySource, LoopOp, ProfileLoop, RepeatMode, Sector};
use scenevm::GeoId;
use std::str::FromStr;
use uuid::Uuid;
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

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
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
                let extrude_abs = surface.extrusion.depth.abs();
                let (base_holes, feature_loops) =
                    split_loops_for_base(&outer_loop, &hole_loops, extrude_abs);
                if dbg {
                    println!(
                        "[DBG] classification: base_holes={}, feature_loops={}",
                        base_holes.len(),
                        feature_loops.len()
                    );
                }

                // 1) BASE WALL from profile loops (outer with holes)
                let mut outer_path = outer_loop.path.clone();

                // Helper: read profile_target for a loop (profile sector → host fallback)
                let loop_profile_target = |pl: &ProfileLoop| -> i32 {
                    if let Some(origin) = pl.origin_profile_sector {
                        if let Some(profile_id) = surface.profile {
                            if let Some(profile_map) = map.profiles.get(&profile_id) {
                                if let Some(ps) = profile_map.find_sector(origin) {
                                    return ps.properties.get_int_default("profile_target", 0);
                                }
                            }
                        }
                    }
                    sector.properties.get_int_default("profile_target", 0)
                };

                // Start with true base holes (cutouts + through recesses)
                let mut holes_paths: Vec<Vec<vek::Vec2<f32>>> =
                    base_holes.iter().map(|h| h.path.clone()).collect();

                // Symmetry: if extruded, also cut holes on the FRONT cap for shallow recesses
                // that explicitly target the FRONT (profile_target == 0). This makes the pocket visible
                // from the front when editing recess-on-front.
                if surface.extrusion.enabled && extrude_abs > 1e-6 {
                    for h in &hole_loops {
                        if loop_profile_target(h) == 0 {
                            if let LoopOp::Recess { depth: d } = h.op {
                                if d + 1e-5f32 < extrude_abs {
                                    holes_paths.push(h.path.clone());
                                }
                            }
                        }
                    }
                }

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
                                .map(|m| chunk.add_shader(&m.build_shader(), assets))
                        })
                        .flatten();
                    #[derive(Clone, Copy)]
                    enum MaterialKind {
                        Cap,
                        Side,
                    }

                    // Helper function (no captures): push a batch with sector material. Side prefers `side_source`.
                    fn push_with_material_kind_local(
                        kind: MaterialKind,
                        sector: &Sector,
                        _shader_index: Option<usize>,
                        assets: &Assets,
                        chunk: &mut Chunk,
                        vmchunk: &mut scenevm::Chunk,
                        verts: Vec<[f32; 4]>,
                        inds: Vec<(usize, usize, usize)>,
                        uvs_in: Vec<[f32; 2]>,
                    ) {
                        let mut batch = Batch3D::new(verts.clone(), inds.clone(), uvs_in.clone())
                            .repeat_mode(RepeatMode::RepeatXY)
                            .geometry_source(GeometrySource::Sector(sector.id));

                        let source_key = match kind {
                            MaterialKind::Side => "side_source",
                            MaterialKind::Cap => "source",
                        };
                        let fallback_key = "source";

                        let mut added = false;
                        if let Some(Value::Source(pixelsource)) = sector
                            .properties
                            .get(source_key)
                            .or_else(|| sector.properties.get(fallback_key))
                        {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                // vmchunk.add_poly_3d(
                                //     GeoId::Sector(sector.id),
                                //     tile.id,
                                //     verts.clone(),
                                //     uvs_in.clone(),
                                //     inds.clone(),
                                //     0,
                                //     true,
                                //     None,
                                // );
                                // added = true;

                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    batch.source = PixelSource::StaticTileIndex(texture_index);
                                }
                            }
                        }

                        if !added {
                            vmchunk.add_poly_3d(
                                GeoId::Sector(sector.id),
                                Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                                    .ok()
                                    .unwrap(),
                                verts.clone(),
                                uvs_in.clone(),
                                inds.clone(),
                                0,
                                true,
                                None,
                            );
                        }

                        /*
                        if let Some(si) = shader_index {
                            batch.shader = Some(si);
                            if chunk.shaders_with_opacity[si] {
                                chunk.batches3d_opacity.push(batch);
                            } else {
                                if let Some(si) = batch.shader {
                                    if chunk.shaders_with_opacity[si] {
                                        chunk.batches3d_opacity.push(batch);
                                    } else {
                                        chunk.batches3d.push(batch);
                                    }
                                } else {
                                    chunk.batches3d.push(batch);
                                }
                            }
                        } else {
                            if matches!(batch.source, PixelSource::Pixel(_)) {
                                batch.source = PixelSource::Pixel([128, 128, 128, 255]);
                            }
                            if let Some(si) = batch.shader {
                                if chunk.shaders_with_opacity[si] {
                                    chunk.batches3d_opacity.push(batch);
                                } else {
                                    chunk.batches3d.push(batch);
                                }
                            } else {
                                chunk.batches3d.push(batch);
                            }
                        }*/

                        chunk.batches3d.push(batch);
                    }

                    // Build a side band (jamb) with UVs: U=perimeter distance normalized, V=0..1 across depth
                    let build_jamb_uv = |loop_uv: &Vec<vek::Vec2<f32>>,
                                         depth: f32|
                     -> (
                        Vec<[f32; 4]>,
                        Vec<(usize, usize, usize)>,
                        Vec<[f32; 2]>,
                    ) {
                        let m = loop_uv.len();
                        if m < 2 {
                            return (vec![], vec![], vec![]);
                        }

                        let mut front_ws: Vec<vek::Vec3<f32>> = Vec::with_capacity(m);
                        for i in 0..m {
                            let p = surface.uv_to_world(loop_uv[i]);
                            front_ws.push(p);
                        }
                        let mut dists = vec![0.0f32; m + 1];
                        for i in 0..m {
                            let a = front_ws[i];
                            let b = front_ws[(i + 1) % m];
                            dists[i + 1] = dists[i] + (b - a).magnitude();
                        }
                        let perim = dists[m].max(1e-6);

                        // --- UVs: follow sector tiling rules for sides ---
                        let tile_mode_side = sector.properties.get_int_default(
                            "side_tile_mode",
                            sector.properties.get_int_default("tile_mode", 1),
                        );
                        let tex_scale_u = sector.properties.get_float_default(
                            "side_texture_scale_x",
                            sector.properties.get_float_default("texture_scale_x", 1.0),
                        );
                        let tex_scale_v = sector.properties.get_float_default(
                            "side_texture_scale_y",
                            sector.properties.get_float_default("texture_scale_y", 1.0),
                        );
                        let depth_abs = depth.abs().max(1e-6);

                        // Geometry: independent quad per edge (two triangles)
                        let mut verts: Vec<[f32; 4]> = Vec::with_capacity(m * 4);
                        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(m * 4);
                        let mut inds: Vec<(usize, usize, usize)> = Vec::with_capacity(m * 2);

                        // Use surface normal each time so this helper is independent
                        let mut n = surface.plane.normal;
                        let l = n.magnitude();
                        if l > 1e-6 {
                            n /= l;
                        } else {
                            n = vek::Vec3::unit_y();
                        }

                        for i in 0..m {
                            let ia = i;
                            let ib = (i + 1) % m;
                            let a_uv = loop_uv[ia];
                            let b_uv = loop_uv[ib];
                            let a_world = surface.uv_to_world(a_uv);
                            let b_world = surface.uv_to_world(b_uv);
                            let a_back = a_world + n * depth;
                            let b_back = b_world + n * depth;

                            let base = verts.len();
                            verts.push([a_world.x, a_world.y, a_world.z, 1.0]);
                            verts.push([b_world.x, b_world.y, b_world.z, 1.0]);
                            verts.push([b_back.x, b_back.y, b_back.z, 1.0]);
                            verts.push([a_back.x, a_back.y, a_back.z, 1.0]);

                            // U along perimeter, V across depth
                            let ua_raw = dists[ia];
                            let ub_raw = dists[ib];
                            let (ua, ub, v0, v1) = if tile_mode_side == 0 {
                                // Fit: normalize to 0..1 in both axes
                                (ua_raw / perim, ub_raw / perim, 0.0, 1.0)
                            } else {
                                // Repeat: scale in world units by texture scales
                                (
                                    ua_raw / tex_scale_u.max(1e-6),
                                    ub_raw / tex_scale_u.max(1e-6),
                                    0.0,
                                    depth_abs / tex_scale_v.max(1e-6),
                                )
                            };
                            uvs.push([ua, v0]);
                            uvs.push([ub, v0]);
                            uvs.push([ub, v1]);
                            uvs.push([ua, v1]);

                            inds.push((base + 0, base + 1, base + 2));
                            inds.push((base + 0, base + 2, base + 3));
                        }

                        (verts, inds, uvs)
                    };

                    push_with_material_kind_local(
                        MaterialKind::Cap,
                        sector,
                        shader_index,
                        assets,
                        chunk,
                        vmchunk,
                        world_vertices.clone(),
                        indices.clone(),
                        uvs.clone(),
                    );

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

                        // 1) Back cap at z = depth (offset along normal), with its OWN holes
                        {
                            // Helper: read profile_target for a loop (profile sector → host fallback)
                            let loop_profile_target = |pl: &ProfileLoop| -> i32 {
                                if let Some(origin) = pl.origin_profile_sector {
                                    if let Some(profile_id) = surface.profile {
                                        if let Some(profile_map) = map.profiles.get(&profile_id) {
                                            if let Some(ps) = profile_map.find_sector(origin) {
                                                return ps
                                                    .properties
                                                    .get_int_default("profile_target", 0);
                                            }
                                        }
                                    }
                                }
                                sector.properties.get_int_default("profile_target", 0)
                            };

                            // Decide which holes must be subtracted from the BACK cap:
                            //  - pure cutouts (None)
                            //  - through recesses (depth >= thickness)
                            //  - shallow recesses that TARGET THE BACK CAP (profile_target==1)
                            let eps = 1e-5f32;
                            let extrude_abs = surface.extrusion.depth.abs();
                            let mut back_holes_paths: Vec<Vec<vek::Vec2<f32>>> = Vec::new();
                            for h in &hole_loops {
                                let to_back = loop_profile_target(h) == 1;
                                match h.op {
                                    LoopOp::None => {
                                        back_holes_paths.push(h.path.clone());
                                    }
                                    LoopOp::Recess { depth: d } => {
                                        if d + eps >= extrude_abs {
                                            // through recess → hole on both caps
                                            back_holes_paths.push(h.path.clone());
                                        } else if to_back {
                                            // shallow recess targeted to the BACK cap → visible pocket → cut a hole on BACK
                                            back_holes_paths.push(h.path.clone());
                                        }
                                    }
                                    LoopOp::Relief { .. } => { /* no hole */ }
                                }
                            }

                            // Triangulate back cap with its holes
                            let mut back_outer = outer_loop.path.clone();
                            if let Some((back_verts_uv, mut back_indices)) =
                                earcut_with_holes(&mut back_outer, &mut back_holes_paths)
                            {
                                // Map UV to world on back plane
                                let back_world_vertices: Vec<[f32; 4]> = back_verts_uv
                                    .iter()
                                    .map(|uv| {
                                        let p = surface.uv_to_world(vek::Vec2::new(uv[0], uv[1]))
                                            + n * depth;
                                        [p.x, p.y, p.z, 1.0]
                                    })
                                    .collect();

                                // Faces should point opposite to front cap on the back
                                fix_winding(
                                    &back_world_vertices,
                                    &mut back_indices,
                                    -surface.plane.normal,
                                );

                                // Build UVs same as front (scale/tiling based on sector props)
                                let tile_mode = sector.properties.get_int_default("tile_mode", 1);
                                let mut minx = f32::INFINITY;
                                let mut miny = f32::INFINITY;
                                let mut maxx = f32::NEG_INFINITY;
                                let mut maxy = f32::NEG_INFINITY;
                                for v in &back_verts_uv {
                                    minx = minx.min(v[0]);
                                    maxx = maxx.max(v[0]);
                                    miny = miny.min(v[1]);
                                    maxy = maxy.max(v[1]);
                                }
                                let sx = (maxx - minx).max(1e-6);
                                let sy = (maxy - miny).max(1e-6);
                                let mut back_uvs: Vec<[f32; 2]> =
                                    Vec::with_capacity(back_verts_uv.len());
                                if tile_mode == 0 {
                                    for v in &back_verts_uv {
                                        back_uvs.push([(v[0] - minx) / sx, (v[1] - miny) / sy]);
                                    }
                                } else {
                                    let tex_scale_x =
                                        sector.properties.get_float_default("texture_scale_x", 1.0);
                                    let tex_scale_y =
                                        sector.properties.get_float_default("texture_scale_y", 1.0);
                                    for v in &back_verts_uv {
                                        back_uvs.push([
                                            (v[0] - minx) / tex_scale_x,
                                            (v[1] - miny) / tex_scale_y,
                                        ]);
                                    }
                                }

                                push_with_material_kind_local(
                                    MaterialKind::Cap,
                                    sector,
                                    shader_index,
                                    assets,
                                    chunk,
                                    vmchunk,
                                    back_world_vertices,
                                    back_indices,
                                    back_uvs,
                                );
                            }
                        }

                        // Helper to push a side band (outer ring or through-hole tube)
                        let mut push_side_band = |loop_uv: &Vec<vek::Vec2<f32>>| {
                            let (ring_v, mut ring_i, ring_uv) = build_jamb_uv(loop_uv, depth);
                            fix_winding(&ring_v, &mut ring_i, surface.plane.normal);
                            push_with_material_kind_local(
                                MaterialKind::Side,
                                sector,
                                shader_index,
                                assets,
                                chunk,
                                vmchunk,
                                ring_v,
                                ring_i,
                                ring_uv,
                            );
                        };

                        // 2) Outer perimeter side band
                        push_side_band(&outer_loop.path);

                        // 3) Through-hole tubes for **actual** base holes (cutouts + through-recesses)
                        for h in &base_holes {
                            push_side_band(&h.path);
                        }
                    }
                }

                // 2) FEATURE LOOPS: build caps + jambs
                for fl in feature_loops {
                    // Helper: read an int property from the originating profile sector; fallback to host sector
                    let feature_int_default =
                        |key: &str, origin: Option<u32>, default: i32| -> i32 {
                            if let (Some(profile_id), Some(origin_id)) = (surface.profile, origin) {
                                if let Some(profile_map) = map.profiles.get(&profile_id) {
                                    if let Some(ps) = profile_map.find_sector(origin_id) {
                                        return ps.properties.get_int_default(key, default);
                                    }
                                }
                            }
                            sector.properties.get_int_default(key, default)
                        };
                    // Helper: read a float property from the originating profile sector; fallback to host sector
                    let feature_float_default =
                        |key: &str, origin: Option<u32>, default: f32| -> f32 {
                            if let (Some(profile_id), Some(origin_id)) = (surface.profile, origin) {
                                if let Some(profile_map) = map.profiles.get(&profile_id) {
                                    if let Some(ps) = profile_map.find_sector(origin_id) {
                                        return ps.properties.get_float_default(key, default);
                                    }
                                }
                            }
                            sector.properties.get_float_default(key, default)
                        };
                    // Local UV jamb builder for feature loops (U = perimeter distance, V = depth 0..1), starting from a base plane offset
                    let build_jamb_uv = |loop_uv: &Vec<vek::Vec2<f32>>,
                                         base_offset: f32,
                                         depth: f32,
                                         origin: Option<u32>,
                                         prefix: &str|
                     -> (
                        Vec<[f32; 4]>,
                        Vec<(usize, usize, usize)>,
                        Vec<[f32; 2]>,
                    ) {
                        let m = loop_uv.len();
                        if m < 2 {
                            return (vec![], vec![], vec![]);
                        }

                        // Accumulate perimeter distances in **world space** on the selected cap plane
                        let mut n = surface.plane.normal;
                        let l = n.magnitude();
                        if l > 1e-6 {
                            n /= l;
                        } else {
                            n = vek::Vec3::unit_y();
                        }

                        let mut front_ws: Vec<vek::Vec3<f32>> = Vec::with_capacity(m);
                        for i in 0..m {
                            let p = surface.uv_to_world(loop_uv[i]) + n * base_offset;
                            front_ws.push(p);
                        }
                        let mut dists = vec![0.0f32; m + 1];
                        for i in 0..m {
                            let a = front_ws[i];
                            let b = front_ws[(i + 1) % m];
                            dists[i + 1] = dists[i] + (b - a).magnitude();
                        }
                        let perim = dists[m].max(1e-6);

                        // --- UVs: follow sector tiling rules for sides with per-feature overrides ---
                        let tile_mode_side = sector.properties.get_int_default(
                            "side_tile_mode",
                            sector.properties.get_int_default("tile_mode", 1),
                        );
                        // Allow per-feature overrides, e.g. "relief_jamb_texture_scale_x/y" or "recess_jamb_texture_scale_x/y"
                        let ov_u_key = format!("{}_jamb_texture_scale_x", prefix);
                        let ov_v_key = format!("{}_jamb_texture_scale_y", prefix);
                        let ov_u = feature_float_default(&ov_u_key, origin, f32::NAN);
                        let ov_v = feature_float_default(&ov_v_key, origin, f32::NAN);

                        let tex_scale_u = if ov_u.is_nan() {
                            sector.properties.get_float_default(
                                "side_texture_scale_x",
                                sector.properties.get_float_default("texture_scale_x", 1.0),
                            )
                        } else {
                            ov_u
                        };
                        // For RELIEF jambs, default V-scale to the CAP’s scale so vertical tiling matches the face.
                        // For other jambs (e.g. RECESS), keep the side default unless overridden per-feature.
                        let tex_scale_v = if ov_v.is_nan() {
                            if prefix == "relief" {
                                sector.properties.get_float_default("texture_scale_y", 1.0)
                            } else {
                                sector.properties.get_float_default(
                                    "side_texture_scale_y",
                                    sector.properties.get_float_default("texture_scale_y", 1.0),
                                )
                            }
                        } else {
                            ov_v
                        };
                        let depth_abs = depth.abs().max(1e-6);

                        // Surface normal
                        let mut n = surface.plane.normal;
                        let l = n.magnitude();
                        if l > 1e-6 {
                            n /= l;
                        } else {
                            n = vek::Vec3::unit_y();
                        }

                        // Geometry: independent quad per edge (two triangles)
                        let mut verts: Vec<[f32; 4]> = Vec::with_capacity(m * 4);
                        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(m * 4);
                        let mut inds: Vec<(usize, usize, usize)> = Vec::with_capacity(m * 2);

                        for i in 0..m {
                            let ia = i;
                            let ib = (i + 1) % m;
                            let a_uv = loop_uv[ia];
                            let b_uv = loop_uv[ib];

                            // Start at the chosen base plane (front or back cap), then extrude by `depth`
                            let a_front = surface.uv_to_world(a_uv) + n * base_offset;
                            let b_front = surface.uv_to_world(b_uv) + n * base_offset;
                            let a_back = a_front + n * depth;
                            let b_back = b_front + n * depth;

                            let base = verts.len();
                            verts.push([a_front.x, a_front.y, a_front.z, 1.0]);
                            verts.push([b_front.x, b_front.y, b_front.z, 1.0]);
                            verts.push([b_back.x, b_back.y, b_back.z, 1.0]);
                            verts.push([a_back.x, a_back.y, a_back.z, 1.0]);

                            // U along perimeter, V across depth
                            let ua_raw = dists[ia];
                            let ub_raw = dists[ib];
                            let (ua, ub, v0, v1) = if tile_mode_side == 0 {
                                // Fit: normalize to 0..1 in both axes
                                (ua_raw / perim, ub_raw / perim, 0.0, 1.0)
                            } else {
                                // Repeat: scale in world units by texture scales
                                (
                                    ua_raw / tex_scale_u.max(1e-6),
                                    ub_raw / tex_scale_u.max(1e-6),
                                    0.0,
                                    depth_abs / tex_scale_v.max(1e-6),
                                )
                            };
                            uvs.push([ua, v0]);
                            uvs.push([ub, v0]);
                            uvs.push([ub, v1]);
                            uvs.push([ua, v1]);

                            inds.push((base + 0, base + 1, base + 2));
                            inds.push((base + 0, base + 2, base + 3));
                        }
                        (verts, inds, uvs)
                    };

                    match fl.op {
                        LoopOp::Relief { height } if height > 0.0 => {
                            // Decide which cap to attach this feature to: 0=front, 1=back (per-profile-sector)
                            let profile_target =
                                feature_int_default("profile_target", fl.origin_profile_sector, 0);
                            let mut n = surface.plane.normal;
                            let ln = n.magnitude();
                            if ln > 1e-6 {
                                n /= ln;
                            } else {
                                n = vek::Vec3::unit_y();
                            }

                            let cap_offset = if profile_target == 1 {
                                surface.extrusion.depth
                            } else {
                                0.0
                            };

                            // Relief should bulge **outward** from the chosen cap:
                            // For FRONT (0): outward is along -n  → dir_sign = -1
                            // For BACK  (1): outward is along +n  → dir_sign = +1
                            let dir_sign = if profile_target == 0 { -1.0 } else { 1.0 };

                            // Place relief cap by shifting from chosen cap plane along dir_sign * height
                            let offset_scalar = cap_offset + dir_sign * height;
                            if let Some((cap_v, cap_i, cap_uv)) =
                                build_cap(surface, &fl.path, offset_scalar)
                            {
                                let mut cap_i = cap_i;
                                let desired_n: Vec3<f32> = if profile_target == 0 { -n } else { n }; // face outward
                                fix_winding(&cap_v, &mut cap_i, desired_n);
                                let mut batch =
                                    Batch3D::new(cap_v.clone(), cap_i.clone(), cap_uv.clone())
                                        .repeat_mode(RepeatMode::RepeatXY)
                                        .geometry_source(GeometrySource::Sector(sector.id));
                                if let Some(si) = feature_shader_index(
                                    surface,
                                    map,
                                    sector,
                                    fl.origin_profile_sector,
                                    chunk,
                                    assets,
                                ) {
                                    batch.shader = Some(si);
                                }

                                let mut added = false;

                                if let Some(Value::Source(pixelsource)) = feature_pixelsource(
                                    surface,
                                    map,
                                    sector,
                                    fl.origin_profile_sector,
                                    "relief_source",
                                ) {
                                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                        // vmchunk.add_poly_3d(
                                        //     GeoId::Sector(sector.id),
                                        //     tile.id,
                                        //     cap_v.clone(),
                                        //     cap_uv.clone(),
                                        //     cap_i.clone(),
                                        //     0,
                                        //     true,
                                        //     None,
                                        // );
                                        // added = true;

                                        if let Some(tex) = assets.tile_index(&tile.id) {
                                            batch.source = PixelSource::StaticTileIndex(tex);
                                        }
                                    }
                                }
                                if !added {
                                    vmchunk.add_poly_3d(
                                        GeoId::Sector(sector.id),
                                        Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                                            .ok()
                                            .unwrap(),
                                        cap_v,
                                        cap_uv,
                                        cap_i,
                                        0,
                                        true,
                                        None,
                                    );
                                }
                                /*
                                // Inherit host material if nothing set by per-feature keys
                                if matches!(batch.source, PixelSource::Pixel(_)) {
                                    if let Some(Value::Source(pixelsource)) =
                                        sector.properties.get("source")
                                    {
                                        if let Some(tile) = pixelsource.tile_from_tile_list(assets)
                                        {
                                            if let Some(tex) = assets.tile_index(&tile.id) {
                                                batch.source = PixelSource::StaticTileIndex(tex);
                                            }
                                        }
                                    }
                                }
                                if let Some(si) = batch.shader {
                                    if chunk.shaders_with_opacity[si] {
                                        chunk.batches3d_opacity.push(batch);
                                    } else {
                                        chunk.batches3d.push(batch);
                                    }
                                } else {
                                    chunk.batches3d.push(batch);
                                }*/
                                chunk.batches3d.push(batch);
                            }
                            let (ring_v, ring_i, ring_uv) = build_jamb_uv(
                                &fl.path,
                                cap_offset,
                                dir_sign * height,
                                fl.origin_profile_sector,
                                "relief",
                            );
                            let mut ring_i = ring_i;
                            fix_winding(&ring_v, &mut ring_i, surface.plane.normal);
                            let mut batch =
                                Batch3D::new(ring_v.clone(), ring_i.clone(), ring_uv.clone())
                                    .repeat_mode(RepeatMode::RepeatXY)
                                    .geometry_source(GeometrySource::Sector(sector.id));
                            // if let Some(si) = feature_shader_index(
                            //     surface,
                            //     map,
                            //     sector,
                            //     fl.origin_profile_sector,
                            //     chunk,
                            //     assets,
                            // ) {
                            //     batch.shader = Some(si);
                            // }
                            let mut added = false;
                            if let Some(Value::Source(pixelsource)) = feature_pixelsource(
                                surface,
                                map,
                                sector,
                                fl.origin_profile_sector,
                                "relief_jamb_source",
                            ) {
                                if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                    // vmchunk.add_poly_3d(
                                    //     GeoId::Sector(sector.id),
                                    //     tile.id,
                                    //     ring_v.clone(),
                                    //     ring_uv.clone(),
                                    //     ring_i.clone(),
                                    //     0,
                                    //     true,
                                    //     None,
                                    // );
                                    // added = true;

                                    if let Some(tex) = assets.tile_index(&tile.id) {
                                        batch.source = PixelSource::StaticTileIndex(tex);
                                    }
                                }
                            }
                            if !added {
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                                        .ok()
                                        .unwrap(),
                                    ring_v,
                                    ring_uv,
                                    ring_i,
                                    0,
                                    true,
                                    None,
                                );
                            }
                            /*
                            // Inherit host material if nothing set by per-feature keys
                            if matches!(batch.source, PixelSource::Pixel(_)) {
                                if let Some(Value::Source(pixelsource)) =
                                    sector.properties.get("source")
                                {
                                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                        if let Some(tex) = assets.tile_index(&tile.id) {
                                            batch.source = PixelSource::StaticTileIndex(tex);
                                        }
                                    }
                                }
                            }
                            if let Some(si) = batch.shader {
                                if chunk.shaders_with_opacity[si] {
                                    chunk.batches3d_opacity.push(batch);
                                } else {
                                    chunk.batches3d.push(batch);
                                }
                            } else {
                                chunk.batches3d.push(batch);
                            }*/
                            chunk.batches3d.push(batch);
                        }
                        LoopOp::Recess { depth } if depth > 0.0 => {
                            // Determine which cap we recess into and the direction for that cap
                            let profile_target =
                                feature_int_default("profile_target", fl.origin_profile_sector, 0);
                            let mut n = surface.plane.normal;
                            let ln = n.magnitude();
                            if ln > 1e-6 {
                                n /= ln;
                            } else {
                                n = vek::Vec3::unit_y();
                            }

                            // Cap selection: front (0) or back (1)
                            let cap_offset = if profile_target == 1 {
                                surface.extrusion.depth
                            } else {
                                0.0
                            };

                            // FRONT (0): recess must move **toward back**, i.e. along +n by `depth`
                            // BACK  (1): recess must move **toward front**, i.e. along -n by `depth`
                            let dir_sign = if profile_target == 0 { 1.0 } else { -1.0 };

                            // --- Cap (recess plane) ---
                            // Place recess cap by shifting from chosen cap plane along dir_sign * depth
                            let offset_scalar = cap_offset + dir_sign * depth;
                            if let Some((cap_v, cap_i, cap_uv)) =
                                build_cap(surface, &fl.path, offset_scalar)
                            {
                                let mut cap_i = cap_i;
                                // Recess cap faces into the pocket: opposite of the shift direction
                                let desired_n: Vec3<f32> = if profile_target == 0 { -n } else { n };
                                fix_winding(&cap_v, &mut cap_i, desired_n);
                                if dbg {
                                    println!(
                                        "[DBG] recess cap: verts={}, tris={}, target={} (0=front,1=back)",
                                        cap_v.len(),
                                        cap_i.len(),
                                        profile_target
                                    );
                                }
                                let mut batch =
                                    Batch3D::new(cap_v.clone(), cap_i.clone(), cap_uv.clone())
                                        .repeat_mode(RepeatMode::RepeatXY)
                                        .geometry_source(GeometrySource::Sector(sector.id));
                                // if let Some(si) = feature_shader_index(
                                //     surface,
                                //     map,
                                //     sector,
                                //     fl.origin_profile_sector,
                                //     chunk,
                                //     assets,
                                // ) {
                                //     batch.shader = Some(si);
                                // }
                                let mut added = false;
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
                                // Inherit host material if nothing set by per-feature keys
                                if matches!(batch.source, PixelSource::Pixel(_)) {
                                    if let Some(Value::Source(pixelsource)) =
                                        sector.properties.get("source")
                                    {
                                        if let Some(tile) = pixelsource.tile_from_tile_list(assets)
                                        {
                                            // vmchunk.add_poly_3d(
                                            //     GeoId::Sector(sector.id),
                                            //     tile.id,
                                            //     cap_v.clone(),
                                            //     cap_uv.clone(),
                                            //     cap_i.clone(),
                                            //     0,
                                            //     true,
                                            //     None,
                                            // );
                                            // added = true;
                                            if let Some(tex) = assets.tile_index(&tile.id) {
                                                batch.source = PixelSource::StaticTileIndex(tex);
                                            }
                                        }
                                    }
                                }
                                if !added {
                                    vmchunk.add_poly_3d(
                                        GeoId::Sector(sector.id),
                                        Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                                            .ok()
                                            .unwrap(),
                                        cap_v,
                                        cap_uv,
                                        cap_i,
                                        0,
                                        true,
                                        None,
                                    );
                                }
                                /*
                                if let Some(si) = batch.shader {
                                    if chunk.shaders_with_opacity[si] {
                                        chunk.batches3d_opacity.push(batch);
                                    } else {
                                        chunk.batches3d.push(batch);
                                    }
                                } else {
                                    chunk.batches3d.push(batch);
                                }*/
                                chunk.batches3d.push(batch);
                            }

                            // --- Jamb (side band into the pocket) ---
                            // build_jamb_uv extrudes along +n by the provided depth from base_offset.
                            // Use the same signed direction as the cap shift
                            let (ring_v, ring_i, ring_uv) = build_jamb_uv(
                                &fl.path,
                                cap_offset,
                                dir_sign * depth,
                                fl.origin_profile_sector,
                                "recess",
                            );
                            let mut ring_i = ring_i;
                            // fix_winding(&ring_v, &mut ring_i, surface.plane.normal);
                            let mut batch =
                                Batch3D::new(ring_v.clone(), ring_i.clone(), ring_uv.clone())
                                    .repeat_mode(RepeatMode::RepeatXY)
                                    .geometry_source(GeometrySource::Sector(sector.id));
                            // if let Some(si) = feature_shader_index(
                            //     surface,
                            //     map,
                            //     sector,
                            //     fl.origin_profile_sector,
                            //     chunk,
                            //     assets,
                            // ) {
                            //     batch.shader = Some(si);
                            // }
                            let mut added = false;
                            if let Some(Value::Source(pixelsource)) = feature_pixelsource(
                                surface,
                                map,
                                sector,
                                fl.origin_profile_sector,
                                "recess_jamb_source",
                            ) {
                                if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                    // vmchunk.add_poly_3d(
                                    //     GeoId::Sector(sector.id),
                                    //     tile.id,
                                    //     ring_v.clone(),
                                    //     ring_uv.clone(),
                                    //     ring_i.clone(),
                                    //     0,
                                    //     true,
                                    //     None,
                                    // );
                                    // added = true;
                                    if let Some(tex) = assets.tile_index(&tile.id) {
                                        batch.source = PixelSource::StaticTileIndex(tex);
                                    }
                                }
                            }
                            if !added {
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                                        .ok()
                                        .unwrap(),
                                    ring_v,
                                    ring_uv,
                                    ring_i,
                                    0,
                                    true,
                                    None,
                                );
                            }
                            // Inherit host material if nothing set by per-feature keys
                            if matches!(batch.source, PixelSource::Pixel(_)) {
                                if let Some(Value::Source(pixelsource)) =
                                    sector.properties.get("source")
                                {
                                    if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                        if let Some(tex) = assets.tile_index(&tile.id) {
                                            batch.source = PixelSource::StaticTileIndex(tex);
                                        }
                                    }
                                }
                            }
                            // if let Some(si) = batch.shader {
                            //     if chunk.shaders_with_opacity[si] {
                            //         chunk.batches3d_opacity.push(batch);
                            //     } else {
                            //         chunk.batches3d.push(batch);
                            //     }
                            // } else {
                            //     chunk.batches3d.push(batch);
                            // }
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
                                .map(|m| chunk.add_shader(&m.build_shader(), assets))
                        })
                        .flatten();

                    #[allow(dead_code)]
                    #[derive(Clone, Copy)]
                    enum MaterialKind {
                        Cap,
                        Side,
                    }

                    // Helper function (no captures): push a batch with sector material. Side prefers `side_source`.
                    fn push_with_material_kind_local(
                        kind: MaterialKind,
                        sector: &Sector,
                        _shader_index: Option<usize>,
                        assets: &Assets,
                        chunk: &mut Chunk,
                        vmchunk: &mut scenevm::Chunk,
                        verts: Vec<[f32; 4]>,
                        inds: Vec<(usize, usize, usize)>,
                        uvs_in: Vec<[f32; 2]>,
                    ) {
                        let mut batch = Batch3D::new(verts.clone(), inds.clone(), uvs_in.clone())
                            .repeat_mode(RepeatMode::RepeatXY)
                            .geometry_source(GeometrySource::Sector(sector.id));

                        let source_key = match kind {
                            MaterialKind::Side => "side_source",
                            MaterialKind::Cap => "source",
                        };
                        let fallback_key = "source";

                        let mut added = false;

                        if let Some(Value::Source(pixelsource)) = sector
                            .properties
                            .get(source_key)
                            .or_else(|| sector.properties.get(fallback_key))
                        {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                // vmchunk.add_poly_3d(
                                //     GeoId::Sector(sector.id),
                                //     tile.id,
                                //     verts.clone(),
                                //     uvs_in.clone(),
                                //     inds.clone(),
                                //     0,
                                //     true,
                                //     None,
                                // );
                                // added = true;
                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    batch.source = PixelSource::StaticTileIndex(texture_index);
                                }
                            }
                        }

                        if !added {
                            vmchunk.add_poly_3d(
                                GeoId::Sector(sector.id),
                                Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                                    .ok()
                                    .unwrap(),
                                verts,
                                uvs_in,
                                inds,
                                0,
                                true,
                                None,
                            );
                        }

                        // if let Some(si) = shader_index {
                        //     batch.shader = Some(si);
                        //     if chunk.shaders_with_opacity[si] {
                        //         chunk.batches3d_opacity.push(batch);
                        //     } else {
                        //         chunk.batches3d.push(batch);
                        //     }
                        // } else {
                        //     if matches!(batch.source, PixelSource::Pixel(_)) {
                        //         batch.source = PixelSource::Pixel([128, 128, 128, 255]);
                        //     }
                        //     chunk.batches3d.push(batch);
                        // }
                        chunk.batches3d.push(batch);
                    }

                    push_with_material_kind_local(
                        MaterialKind::Cap,
                        sector,
                        shader_index,
                        assets,
                        chunk,
                        vmchunk,
                        world_vertices,
                        indices,
                        uvs,
                    );
                }
            }
        }
    }
}

// --- Relief/recess pipeline helpers ---
/// Classify profile loops: only true holes (cutouts and through-recesses) are subtracted from the base;
/// shallow recesses and reliefs are handled as feature meshes.
fn split_loops_for_base<'a>(
    _outer: &'a ProfileLoop,
    holes: &'a [ProfileLoop],
    extrude_depth_abs: f32,
) -> (Vec<&'a ProfileLoop>, Vec<&'a ProfileLoop>) {
    let mut base_holes = Vec::new();
    let mut feature_loops = Vec::new();
    let eps = 1e-5f32;
    for h in holes {
        match h.op {
            LoopOp::None => {
                // Pure cutout → subtract from base; no feature meshes needed
                base_holes.push(h);
            }
            LoopOp::Recess { depth } => {
                if extrude_depth_abs <= eps {
                    // Zero-thickness surface: we need a visible hole in the base cap
                    // *and* a recessed pocket (cap + jamb). Put it in **both** buckets.
                    base_holes.push(h); // subtract from base
                    feature_loops.push(h); // build recess cap + jamb
                } else if depth + eps >= extrude_depth_abs {
                    // Through recess on a thick surface ⇒ just a hole; extrusion pass builds tube
                    base_holes.push(h);
                } else {
                    // Shallow recess on a thick surface ⇒ feature only; base stays intact
                    feature_loops.push(h);
                }
            }
            LoopOp::Relief { .. } => {
                // Relief never subtracts from the base; purely additive feature
                feature_loops.push(h);
            }
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
    // Prefer per-feature explicit key on the originating profile sector
    if let (Some(profile_id), Some(origin_id)) = (surface.profile, loop_origin) {
        if let Some(profile_map) = map.profiles.get(&profile_id) {
            if let Some(ps) = profile_map.find_sector(origin_id) {
                // 1) exact key on profile sector
                if let Some(v) = ps.properties.get(key) {
                    return Some(v.clone());
                }
                // 2) generic 'source' on profile sector
                if let Some(v) = ps.properties.get("source") {
                    return Some(v.clone());
                }
            }
        }
    }

    // Fallback chain on host sector:
    // 3) exact key on host
    if let Some(v) = host_sector.properties.get(key) {
        return Some(v.clone());
    }
    // 4) if this looks like a jamb key, try 'side_source'
    if key.contains("jamb") {
        if let Some(v) = host_sector.properties.get("side_source") {
            return Some(v.clone());
        }
    }
    // 5) generic 'source' on host
    host_sector.properties.get("source").cloned()
}

fn feature_shader_index(
    surface: &crate::Surface,
    map: &Map,
    _host_sector: &Sector,
    loop_origin: Option<u32>,
    chunk: &mut Chunk,
    assets: &Assets,
) -> Option<usize> {
    // Prefer per-feature shader on the originating profile sector
    if let (Some(profile_id), Some(origin_id)) = (surface.profile, loop_origin) {
        if let Some(profile_map) = map.profiles.get(&profile_id) {
            if let Some(ps) = profile_map.find_sector(origin_id) {
                if let Some(shader_id) = ps.shader {
                    if let Some(m) = profile_map.shaders.get(&shader_id) {
                        if let Some(si) = chunk.add_shader(&m.build_shader(), assets) {
                            return Some(si);
                        }
                    }
                }
            }
        }
    }
    /*
    // Fallback to host sector shader
    if let Some(shader_id) = host_sector.shader {
        if let Some(m) = map.shaders.get(&shader_id) {
            return chunk.add_shader(&m.build_shader(), assets);
        }
    }*/
    None
}
