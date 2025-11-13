// --- Test Lambert shading (kept in BODY so headers stay generic) ---
fn lambert_pointlights(P: vec3<f32>, N: vec3<f32>, base_col: vec3<f32>) -> vec3<f32> {
    var diffuse = vec3<f32>(0.0);
    // Use background as ambient; clamp to a small minimum so unlit scenes aren't black
    let ambient = max(U.background.xyz, vec3<f32>(0.05, 0.05, 0.05));

    for (var li: u32 = 0u; li < U.lights_count; li = li + 1u) {
        let light = sd_light(li);

        if (light.header.y == 0u) { continue; } // emitting flag

        let Lp = light.position;
        let Lc = light.color.xyz;
        let Li = light.params0.x + light.params1.x;   // intensity + flicker

        let start_d = light.params0.z;
        let end_d   = max(light.params0.w, start_d + 1e-3);
        let L = Lp.xyz - P;
        let dist2 = max(dot(L, L), 1e-6);
        let dist = sqrt(dist2);
        let Ldir = normalize(L);

        // Always two-sided: use |N·L|
        let ndotl = abs(dot(N, Ldir));

        let fall = clamp((end_d - dist) / max(end_d - start_d, 1e-3), 0.0, 1.0);
        let atten = Li * ndotl * fall / dist2;
        diffuse += Lc * atten;
    }
    return base_col * (ambient + diffuse);
}

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x; let py = gid.y;
    if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

    // Build pixel uv and get ray from the header-provided camera function
    let cam_uv = vec2<f32>( (f32(px) + 0.5) / f32(U.fb_size.x),
                            (f32(py) + 0.5) / f32(U.fb_size.y) );
    let ray = cam_ray(cam_uv);
    let ro = ray.ro;
    let rd = normalize(ray.rd);

    // ===== choose tracing mode =====
    var hit_any = false;
    var best_t  = 1e30;
    var best_tri: u32 = 0u;
    var best_u = 0.0;
    var best_v = 0.0;

    // Try grid first; if it misses, fall back to brute-force triangle loop.
    var grid_used = true;
    if (grid_used) {
        let th = sv_trace_grid(ro, rd, 0.001, 1e6);
        if (th.hit) {
            hit_any = true;
            best_t = th.t;
            best_tri = th.tri;
            best_u = th.u;
            best_v = th.v;
            grid_used = true;
        }
    } else {
    // Brute-force fallback
        let tri_len: u32 = arrayLength(&indices3d.data);
        let tri_count: u32 = tri_len / 3u;
        for (var tri: u32 = 0u; tri < tri_count; tri = tri + 1u) {
            let base = 3u * tri;
            let i0 = indices3d.data[base + 0u];
            let i1 = indices3d.data[base + 1u];
            let i2 = indices3d.data[base + 2u];
            let a = verts3d.data[i0].pos;
            let b = verts3d.data[i1].pos;
            let c = verts3d.data[i2].pos;
            let h = sv_ray_tri_full(ro, rd, a, b, c);
            if (h.hit && h.t < best_t) {
                hit_any = true;
                best_t = h.t;
                best_tri = tri;
                best_u = h.u;
                best_v = h.v;
            }
        }
    }

    if (!hit_any) {
      sv_write(px, py, U.background);
      return;
    }

    // Clamp the winning triangle id against current buffers (defensive)
    let tri_len_elems = arrayLength(&indices3d.data);
    let tri_len = tri_len_elems / 3u;
    let tri_safe = clamp_index_u(best_tri, tri_len);

    let i0 = indices3d.data[3u*tri_safe + 0u];
    let i1 = indices3d.data[3u*tri_safe + 1u];
    let i2 = indices3d.data[3u*tri_safe + 2u];

    let uv0 = verts3d.data[i0].uv; let n0 = verts3d.data[i0].normal;
    let uv1 = verts3d.data[i1].uv; let n1 = verts3d.data[i1].normal;
    let uv2 = verts3d.data[i2].uv; let n2 = verts3d.data[i2].normal;

    let w0 = 1.0 - best_u - best_v;
    // Interpolate smooth normal
    var N = normalize(n0*w0 + n1*best_u + n2*best_v);

    let P = ro + rd * best_t;
    let uv_atlas = sv_tri_atlas_uv_obj(i0, i1, i2, best_u, best_v);

    // when filling Compute3DUniforms u:
    // self.gp8.x = 1.0; // bump strength (0 = off)
    // self.gp9.x = 1.0 / (self.atlas.width as f32);
    // self.gp9.y = 1.0 / (self.atlas.height as f32);

    // Optional bump from the polygon's own texture as height
    if (U.gp8.x > 0.0) {
        // Reconstruct triangle positions for TBN
        let a = verts3d.data[i0].pos;
        let b = verts3d.data[i1].pos;
        let c = verts3d.data[i2].pos;

        // 1 texel steps in atlas UV space (provided by CPU in gp9)
        let du = vec2<f32>(U.gp9.x, 0.0);
        let dv = vec2<f32>(0.0, U.gp9.y);

        // Sample height at uv and neighbors (use color luma as height proxy)
        let h  = sv_luma(textureSampleLevel(atlas_tex, atlas_smp, uv_atlas, 0.0).xyz);
        let hx = sv_luma(textureSampleLevel(atlas_tex, atlas_smp, uv_atlas + du, 0.0).xyz);
        let hy = sv_luma(textureSampleLevel(atlas_tex, atlas_smp, uv_atlas + dv, 0.0).xyz);

        let dhdu = (hx - h);
        let dhdv = (hy - h);

        let TBN = sv_tri_tbn(a, b, c, uv0, uv1, uv2);
        let n_ts = normalize(vec3<f32>(-dhdu * U.gp8.x, -dhdv * U.gp8.x, 1.0));
        let n_ws = normalize(TBN * n_ts);
        N = normalize(mix(N, n_ws, clamp(U.gp8.x, 0.0, 1.0)));
    }

    // If texture/atlas is misbound, use bary-debug color so we still see something
    var base_col = sv_tri_sample_albedo(i0, i1, i2, best_u, best_v);
    if (dot(N, rd) > 0.0) { N = -N; } // two-sided

    // Sample material properties (R/M/O/E) from the secondary atlas
    let mats = sv_tri_sample_rmoe(i0, i1, i2, best_u, best_v);
    let opacity = mats.z;
    let emission = mats.w;

    let base_rgb = base_col.xyz;

    let lit = lambert_pointlights(P, N, base_rgb);
    // Add simple emission, apply opacity (from material)
    let final_rgb = lit;// + M.rmoe.w * M.tint.xyz;
    let final_a = base_col.a;//max(base_col.a * M.rmoe.z, 1.0); // keep visible while debugging

    sv_write(px, py, vec4<f32>(base_col.xyz, final_a));
}
