struct ShadeOut {
    color: vec4<f32>,
    hit: u32,
}

fn sv_shade_one(px: u32, py: u32, p: vec2<f32>) -> ShadeOut {
    let tid = tile_of_px(px, py);
    let ch = sv_shade_tile_pixel(p, px, py, tid);

    if (!ch.hit) {
        return ShadeOut(U.background, 0u);
    }

    // sv_write(px, py, vec4<f32>(1.0));

    return ShadeOut(ch.color, 1u);
}

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;

    if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

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
            if (out.hit != 0u) {
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
        if (out.hit != 0u) {
            // sv_write(px, py, out.color);
        }
    }
}
