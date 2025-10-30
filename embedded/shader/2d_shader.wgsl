struct ShadeOut {
    hit: bool,
    color: vec4<f32>,
}

// Return the final shaded color at subpixel position `p` (in screen pixels).
fn sv_shade_one(px: u32, py: u32, p: vec2<f32>) -> ShadeOut {
    let tid = tile_of_px(px, py);
    let ch = sv_shade_tile_pixel(p, px, py, tid);
    if (!ch.hit) {
        // No scene hit at this sub-sample; don't overwrite the grid/background.
        return ShadeOut(false, U.background);
    }

    // Material look-up for winning triangle
    let m_idx = tri_mat2d.data[ch.tri];
    let M = materials.data[m_idx];

    // Base texture color
    let base = ch.color;
    let base_rgb = base.xyz * M.tint.xyz;

    // Flat material
    if (M.model.x > 0.5) {
        let final_rgb = base_rgb + M.rmoe.w * M.tint.xyz;
        let final_a   = base.a * M.rmoe.z;
        return ShadeOut(true, vec4<f32>(final_rgb, final_a));
    }

    // Ambient term passed via U.gp1 (vec4) — use only RGB
    var ambient_rgb = U.gp1.xyz;

    // Simple 2D lighting: treat surface as facing camera (N = +Z), compute in world plane
    let Pw = sv_world_from_screen(p);
    let P3 = vec3<f32>(Pw.x, 0.0, Pw.y);

    // Accumulate diffuse from point lights
    var diffuse = vec3<f32>(0.0);
    for (var li: u32 = 0u; li < U.lights_count; li = li + 1u) {
        if (lights.data[li].header.y == 0u) { continue; }

        let Lp = lights.data[li].position;
        let Lc = lights.data[li].color.xyz;
        let Li = lights.data[li].params0.x * lights.data[li].params1.x; // Intensity * flicker

        let start_d = lights.data[li].params0.z;
        let end_d   = max(lights.data[li].params0.w, start_d + 1e-3);

        let L = Lp.xyz - P3;
        let dist2 = max(dot(L, L), 1e-6);
        let dist = sqrt(dist2);
        let fall = clamp((end_d - dist) / max(end_d - start_d, 1e-3), 0.0, 1.0);

        // Two-sided for 2D (flat normal)
        let ndotl = 1.0;

        let atten = Li * ndotl * fall / dist2;
        diffuse += Lc * atten;
    }

    // Combine ambient + diffuse; add emission; apply opacity
    let lit = base_rgb * clamp(ambient_rgb + diffuse, vec3<f32>(0.0), vec3<f32>(1.0));
    let final_rgb = lit + M.rmoe.w * M.tint.xyz;
    let final_a   = base.a * M.rmoe.z;

    return ShadeOut(true, vec4<f32>(final_rgb, final_a));
}

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;
    
    if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

    // Clear to background first
    sv_write(px, py, U.background);

    // Draw grid if enabled
    if (U.gp0.x > 0.0) {
        // Grid parameters encoded in U.gp0:
        // x = grid_size (pixels), y = subdivisions, z = offset.x, w = offset.y
        let grid_size_px   = U.gp0.x;
        let subdivisions_f = max(1.0, round(U.gp0.y));
        let offset         = vec2<f32>(U.gp0.z, U.gp0.w);

        // Screen size in pixels
        let screen = vec2<f32>(f32(U.fb_size.x), f32(U.fb_size.y));

        // Pixel center in screen space
        let pos = vec2<f32>(f32(px) + 0.5, f32(py) + 0.5);

        // Origin of the grid (screen center + offset), aligned to whole pixels (odd thickness)
        let origin = screen * 0.5 + offset;
        let aligned_origin = round(origin - vec2<f32>(0.5, 0.5)) + vec2<f32>(0.5, 0.5);

        // Helpers implemented inline
        let grid_size = vec2<f32>(grid_size_px, grid_size_px);

        // Relative position from origin
        let rel_p = pos - aligned_origin;

        // Distance to nearest main grid line along each axis:
        // mul_dist(delta, value) = abs(value - delta * round(value / delta))
        let closest_mul_main = grid_size * round(rel_p / grid_size);
        let dist = abs(rel_p - closest_mul_main);

        // Colors (match Eldiron defaults)
        let bg_color       = vec4<f32>(0.05, 0.05, 0.05, 1.0);
        let line_color     = vec4<f32>(0.15, 0.15, 0.15, 1.0);
        let sub_line_color = vec4<f32>(0.11, 0.11, 0.11, 1.0);

        // Thickness in pixels (use 1px for both major and minor)
        let th  = 1.0;
        let sth = 1.0;

        // Compute grid color for this pixel
        var grid_col = bg_color;

        // Main grid?
        if (min(dist.x, dist.y) <= th * 0.5) {
            grid_col = line_color;
        } else {
            // Compute distance to nearest subdivision line.
            // sub_size = grid_size / round(subdivisions)
            let sub_div = vec2<f32>(subdivisions_f, subdivisions_f);
            let sub_size = grid_size / sub_div;

            // Distance to floor within the main cell
            let dist_to_floor = abs(rel_p - grid_size * floor(rel_p / grid_size));

            // Distance to nearest sub-grid line within the main cell
            let closest_mul_sub = sub_size * round(dist_to_floor / sub_size);
            var sub_dist = abs(dist_to_floor - closest_mul_sub);

            // Number of sub-cells along each axis from the main line
            let rc = round(dist / sub_size);

            // Extra pixels for the last row/column to exactly hit the main grid line
            let extra = grid_size - sub_size * sub_div;

            // If we're exactly at the last subdivision cell, add the leftover to sub_dist
            let rc_i = vec2<u32>(u32(round(rc.x)), u32(round(rc.y)));
            let sub_i= vec2<u32>(u32(round(sub_div.x)), u32(round(sub_div.y)));

            if (rc_i.x == sub_i.x) {
                sub_dist = vec2<f32>(sub_dist.x + extra.x, sub_dist.y);
            }
            if (rc_i.y == sub_i.y) {
                sub_dist = vec2<f32>(sub_dist.x, sub_dist.y + extra.y);
            }

            if (min(sub_dist.x, sub_dist.y) <= sth * 0.5) {
                grid_col = sub_line_color;
            }
        }

        // Draw grid as background; scene shading that follows will overwrite this pixel.
        sv_write(px, py, grid_col);
    }

    // Supersampling option: U.gp7.y >= 2.0 -> 2× SSAA with rotated-grid offsets
    let ss_samples = u32(2);

    if (ss_samples >= 2u) {
        let offsets = array<vec2<f32>, 2>(
            vec2<f32>(-0.25, -0.25),
            vec2<f32>( 0.25,  0.25)
        );
        var accum = vec4<f32>(0.0);
        var hits: u32 = 0u;
        for (var s: u32 = 0u; s < 2u; s = s + 1u) {
            let p_sub = vec2<f32>(f32(px) + 0.5 + offsets[s].x,
                                  f32(py) + 0.5 + offsets[s].y);
            let out = sv_shade_one(px, py, p_sub);
            if (out.hit) {
                accum += out.color;
                hits += 1u;
            }
        }
        if (hits > 0u) {
            sv_write(px, py, accum / vec4<f32>(f32(hits)));
        } 
    } else {
        let p0 = vec2<f32>(f32(px) + 0.5, f32(py) + 0.5);
        let out = sv_shade_one(px, py, p0);
        if (out.hit) {
            sv_write(px, py, out.color);
        }
    }
}