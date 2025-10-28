  // --- Test Lambert shading (kept in BODY so headers stay generic) ---
  fn lambert_pointlights(P: vec3<f32>, N: vec3<f32>, base_col: vec3<f32>) -> vec3<f32> {
    var diffuse = vec3<f32>(0.0);
    // Use background as ambient; make it visible out of the box
    let ambient = U.background.xyz;

    for (var li: u32 = 0u; li < U.lights_count; li = li + 1u) {
      if (lights.data[li].header.y == 0u) { continue; } // emitting flag

      let Lp = lights.data[li].position;
      let Lc = lights.data[li].color.xyz;
      let Li = lights.data[li].params0.x + lights.data[li].params1.x;   // intensity + flicker

      let start_d = lights.data[li].params0.z;
      let end_d   = max(lights.data[li].params0.w, start_d + 1e-3);
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

  // Unpack camera (unchanged) ...
  let cam_pos = U.gp0.xyz;
  let fovy = U.gp0.w;
  let dir = normalize(U.gp1.xyz);
  let aspect = U.gp1.w;
  let right = normalize(U.gp2.xyz);
  let up = normalize(U.gp3.xyz);

  let sx = (f32(px) + 0.5) / f32(U.fb_size.x);
  let sy = (f32(py) + 0.5) / f32(U.fb_size.y);
  let x_ndc = 2.0 * sx - 1.0;
  let y_ndc = 1.0 - 2.0 * sy;
  let tproj = tan(0.5 * fovy);
  let rd = normalize(dir + x_ndc * tproj * aspect * right + y_ndc * tproj * up);
  let ro = cam_pos;

  // ===== choose tracing mode =====
  var hit_any = false;
  var best_t  = 1e30;
  var best_tri: u32 = 0u;
  var best_u = 0.0;
  var best_v = 0.0;

  if (sv_grid_active()) {
    let th = sv_trace_grid(ro, rd, 0.001, 1e6);
    if (th.hit) {
      hit_any = true;
      best_t = th.t;
      best_tri = th.tri;
      best_u = th.u;
      best_v = th.v;
    }
  } else {
    // Brute-force: loop all triangles in indices3d
    let tri_count: u32 = arrayLength(&indices3d.data) / 3u;
    for (var tri: u32 = 0u; tri < tri_count; tri = tri + 1u) {
      let i0 = indices3d.data[3u*tri + 0u];
      let i1 = indices3d.data[3u*tri + 1u];
      let i2 = indices3d.data[3u*tri + 2u];
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

  // Interpolate UV & smooth normal
  let i0 = indices3d.data[3u*best_tri + 0u];
  let i1 = indices3d.data[3u*best_tri + 1u];
  let i2 = indices3d.data[3u*best_tri + 2u];

  let uv0 = verts3d.data[i0].uv; let n0 = verts3d.data[i0].normal;
  let uv1 = verts3d.data[i1].uv; let n1 = verts3d.data[i1].normal;
  let uv2 = verts3d.data[i2].uv; let n2 = verts3d.data[i2].normal;

  let w0 = 1.0 - best_u - best_v;
  let uv = uv0*w0 + uv1*best_u + uv2*best_v;
  var N = normalize(n0*w0 + n1*best_u + n2*best_v);

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

    // 1 texel steps in atlas UV space (provided by CPU)
    let du = vec2<f32>(U.gp9.x, 0.0);
    let dv = vec2<f32>(0.0, U.gp9.y);

    // Sample height at uv and neighbors (use color as height proxy)
    let h  = sv_luma(textureSampleLevel(atlas_tex, atlas_smp, uv, 0.0).xyz);
    let hx = sv_luma(textureSampleLevel(atlas_tex, atlas_smp, uv + du, 0.0).xyz);
    let hy = sv_luma(textureSampleLevel(atlas_tex, atlas_smp, uv + dv, 0.0).xyz);

    // Finite differences
    let dhdu = (hx - h);
    let dhdv = (hy - h);

    // Tangent frame of the triangle
    let TBN = sv_tri_tbn(a, b, c, uv0, uv1, uv2);

    // Map height gradient into tangent space normal and to world space
    let n_ts = normalize(vec3<f32>(-dhdu * U.gp8.x, -dhdv * U.gp8.x, 1.0));
    let n_ws = normalize(TBN * n_ts);

    // Blend with your smooth vertex normal for stability
    N = normalize(mix(N, n_ws, clamp(U.gp8.x, 0.0, 1.0)));
  }  

  let P = ro + rd * best_t;

    let base_col = textureSampleLevel(atlas_tex, atlas_smp, uv, 0.0);
    if (dot(N, rd) > 0.0) { N = -N; } // two-sided

    // Material lookup for the winning triangle
    let m_idx = tri_mat.data[best_tri];
    let M = materials.data[m_idx];
    let base_rgb = base_col.xyz * M.tint.xyz;

    let lit = lambert_pointlights(P, N, base_rgb);
    // Add simple emission, apply opacity (from material)
    let final_rgb = lit + M.rmoe.w * M.tint.xyz;
    let final_a = base_col.a * M.rmoe.z;
    sv_write(px, py, vec4<f32>(final_rgb, final_a));
}