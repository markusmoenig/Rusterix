@compute @workgroup_size(8,8,1)
fn cs_main(
  @builtin(global_invocation_id) gid: vec3<u32>,
  @builtin(workgroup_id) wg: vec3<u32>,
  @builtin(local_invocation_id) lid: vec3<u32>
) {
  let px = wg.x * 8u + lid.x;
  let py = wg.y * 8u + lid.y;
  if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

  let uv = vec2<f32>(f32(gid.x)/f32(U.fb_size.x), f32(gid.y)/f32(U.fb_size.y));

  // Convert pixel coordinates into world coordinates
  //let world_pos = sv_world_from_screen(vec2<f32>(f32(px), f32(py)));  

  let p = vec2<f32>(f32(px) + 0.5, f32(py) + 0.5);

  // Clear to background first
  sv_write(px, py, U.background);

        // let n = U.lights_count;
        // for (var i: u32 = 0u; i < n; i = i + 1u) {
        //     let L = lights.data[i];
        //     // L.position, L.color, L.intensity, etc.
        //     }

  let tid = tile_index(wg.x, wg.y);
  let ch = sv_shade_tile_pixel(p, px, py, tid);
  if (ch.hit) {
    // Example shading hook: modify `ch.color` here if desired.
    let shaded = ch.color; // e.g., multiply by a tint: ch.color * U.gp0;
    sv_write(px, py, shaded);
    return;
  }
}