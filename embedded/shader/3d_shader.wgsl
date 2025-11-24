// PBR Ray-Traced Shader for Eldiron
// Supports: Cook-Torrance BRDF, ray-traced shadows, AO, full material system, and bump mapping
//
// Uniforms (gp0-gp9):
// - gp0.xyz: Sky color (RGB)
// - gp0.w:   Unused
// - gp1.xyz: Sun color (RGB)
// - gp1.w:   Sun intensity
// - gp2.xyz: Sun direction (normalized)
// - gp2.w:   Sun enabled (0.0 = disabled, 1.0 = enabled)
// - gp3.xyz: Ambient color (RGB, independent from sky)
// - gp3.w:   Ambient strength
// - gp4.xyz: Fog color (RGB)
// - gp4.w:   Fog density (0.0 = no fog, higher values = denser fog)

// ===== Constants =====
const PI: f32 = 3.14159265359;
const AO_SAMPLES: u32 = 8u;
const AO_RADIUS: f32 = 0.5;
const BUMP_STRENGTH: f32 = 1.0;
const MIN_ROUGHNESS: f32 = 0.04;
const MAX_TRANSPARENCY_BOUNCES: u32 = 8u;

// ===== Hash functions for random sampling =====
fn hash13(p3: vec3<f32>) -> f32 {
    var p = fract(p3 * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

fn hash33(p3: vec3<f32>) -> vec3<f32> {
    var p = fract(p3 * vec3<f32>(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yxz + 33.33);
    return fract((p.xxy + p.yxx) * p.zyx);
}

// ===== Cosine-weighted hemisphere sampling =====
fn cosine_sample_hemisphere(u1: f32, u2: f32) -> vec3<f32> {
    let r = sqrt(u1);
    let theta = 2.0 * PI * u2;
    let x = r * cos(theta);
    let y = r * sin(theta);
    let z = sqrt(max(0.0, 1.0 - u1));
    return vec3<f32>(x, y, z);
}

// Build orthonormal basis from normal
fn build_onb(N: vec3<f32>) -> mat3x3<f32> {
    let up = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(N.y) > 0.999);
    let T = normalize(cross(up, N));
    let B = cross(N, T);
    return mat3x3<f32>(T, B, N);
}

// ===== PBR Helper Functions =====

// GGX/Trowbridge-Reitz normal distribution function
fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;

    let denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (PI * denom * denom + 1e-7);
}

// Schlick-GGX geometry function (single direction)
fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k + 1e-7);
}

// Smith's method for geometry obstruction
fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = geometry_schlick_ggx(NdotV, roughness);
    let ggx1 = geometry_schlick_ggx(NdotL, roughness);
    return ggx1 * ggx2;
}

// Fresnel-Schlick approximation
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// ===== Material unpacking =====
struct Material {
    roughness: f32,
    metallic: f32,
    opacity: f32,
    emissive: f32,
    normal: vec3<f32>,
};

fn unpack_material(mats: vec4<f32>) -> Material {
    // Convert RGBA back to packed u32
    let packed = u32(mats.x * 255.0) | (u32(mats.y * 255.0) << 8u) |
                 (u32(mats.z * 255.0) << 16u) | (u32(mats.w * 255.0) << 24u);

    // Lower 16 bits: materials (4 bits each)
    let mat_bits = packed & 0xFFFFu;
    let roughness = max(f32(mat_bits & 0xFu) / 15.0, MIN_ROUGHNESS);
    let metallic = f32((mat_bits >> 4u) & 0xFu) / 15.0;
    let opacity = f32((mat_bits >> 8u) & 0xFu) / 15.0;
    let emissive = f32((mat_bits >> 12u) & 0xFu) / 15.0;

    // Upper 16 bits: normal X,Y (8 bits each)
    let norm_bits = (packed >> 16u) & 0xFFFFu;
    let nx = (f32(norm_bits & 0xFFu) / 255.0) * 2.0 - 1.0;
    let ny = (f32((norm_bits >> 8u) & 0xFFu) / 255.0) * 2.0 - 1.0;
    let nz = sqrt(max(0.0, 1.0 - nx * nx - ny * ny));

    return Material(roughness, metallic, opacity, emissive, vec3<f32>(nx, ny, nz));
}

// ===== Ray-traced shadows with opacity support =====
fn trace_shadow(P: vec3<f32>, L: vec3<f32>, max_dist: f32) -> f32 {
    let shadow_bias = 0.01; // Increased from 0.001 to avoid edge artifacts
    var current_pos = P + L * shadow_bias;
    var remaining_dist = max_dist;
    var transparency = 1.0; // Starts fully lit

    // Trace multiple hits to accumulate transparency
    let MAX_SHADOW_STEPS = 8u;
    for (var step: u32 = 0u; step < MAX_SHADOW_STEPS; step = step + 1u) {
        let hit = sv_trace_grid(current_pos, L, 0.0, remaining_dist);

        if (!hit.hit) {
            break; // No more occlusion, light reaches
        }

        // Get material at hit point
        let tri = hit.tri;
        let i0 = indices3d.data[3u * tri + 0u];
        let i1 = indices3d.data[3u * tri + 1u];
        let i2 = indices3d.data[3u * tri + 2u];

        // Sample material to get opacity
        let mat_data = sv_tri_sample_rmoe(i0, i1, i2, hit.u, hit.v);
        let mat = unpack_material(mat_data);

        // Accumulate transparency (opacity reduces light transmission)
        transparency *= (1.0 - mat.opacity);

        // Early exit if fully occluded
        if (transparency < 0.01) {
            return 0.0;
        }

        // Continue ray from just past this hit
        current_pos = current_pos + L * (hit.t + shadow_bias);
        remaining_dist = remaining_dist - hit.t - shadow_bias;

        if (remaining_dist <= 0.0) {
            break;
        }
    }

    return transparency;
}

// ===== Ambient Occlusion with opacity support =====
fn compute_ao(P: vec3<f32>, N: vec3<f32>, seed: vec3<f32>) -> f32 {
    let onb = build_onb(N);
    var occlusion = 0.0;

    for (var i: u32 = 0u; i < AO_SAMPLES; i = i + 1u) {
        let hash_seed = seed + vec3<f32>(f32(i) * 0.1);
        let u1 = hash13(hash_seed);
        let u2 = hash13(hash_seed + vec3<f32>(7.3, 11.7, 13.1));

        // Cosine-weighted hemisphere sample
        let local_dir = cosine_sample_hemisphere(u1, u2);
        let world_dir = onb * local_dir;

        let ao_hit = sv_trace_grid(P + N * 0.001, world_dir, 0.0, AO_RADIUS);
        if (ao_hit.hit) {
            // Get material at hit point to check opacity
            let tri = ao_hit.tri;
            let i0 = indices3d.data[3u * tri + 0u];
            let i1 = indices3d.data[3u * tri + 1u];
            let i2 = indices3d.data[3u * tri + 2u];

            let mat_data = sv_tri_sample_rmoe(i0, i1, i2, ao_hit.u, ao_hit.v);
            let mat = unpack_material(mat_data);

            // Weight by distance - closer occluders contribute more
            let dist_factor = 1.0 - (ao_hit.t / AO_RADIUS);

            // Modulate occlusion by opacity (transparent objects occlude less)
            occlusion += dist_factor * mat.opacity;
        }
    }

    return 1.0 - (occlusion / f32(AO_SAMPLES));
}

// ===== PBR Direct Lighting =====
fn pbr_lighting(P: vec3<f32>, N: vec3<f32>, V: vec3<f32>, albedo: vec3<f32>, mat: Material) -> vec3<f32> {
    var Lo = vec3<f32>(0.0);

    // Base reflectance at zero incidence (for dielectrics use 0.04, metals use albedo)
    let F0 = mix(vec3<f32>(0.04), albedo, mat.metallic);

    // ===== Directional Sun Light =====
    if (U.gp2.w > 0.5) { // Sun enabled
        let sun_dir = normalize(U.gp2.xyz);
        let sun_color = pow(U.gp1.xyz, vec3<f32>(2.2)); // Convert from sRGB to linear
        let sun_intensity = U.gp1.w;

        let L = -sun_dir; // Light direction points FROM surface TO light
        let H = normalize(V + L);

        let NdotL = max(dot(N, L), 0.0);

        if (NdotL > 0.0) {
            // Ray-traced shadow (use large distance for directional light)
            let shadow = trace_shadow(P, L, 1e6);

            if (shadow > 0.01) {
                let radiance = sun_color * sun_intensity * shadow;

                // Cook-Torrance BRDF
                let NdotV = max(dot(N, V), 0.0);
                let NDF = distribution_ggx(N, H, mat.roughness);
                let G = geometry_smith(N, V, L, mat.roughness);
                let F = fresnel_schlick(max(dot(H, V), 0.0), F0);

                let numerator = NDF * G * F;
                let denominator = 4.0 * NdotV * NdotL + 1e-7;
                let specular = numerator / denominator;

                // Energy conservation
                let kS = F;
                let kD = (vec3<f32>(1.0) - kS) * (1.0 - mat.metallic);

                Lo += (kD * albedo / PI + specular) * radiance * NdotL;
            }
        }
    }

    // ===== Point Lights =====
    for (var li: u32 = 0u; li < U.lights_count; li = li + 1u) {
        let light = sd_light(li);

        if (light.header.y == 0u) { continue; } // skip non-emitting lights

        let Lp = light.position.xyz;
        let Lc = pow(light.color.xyz, vec3<f32>(2.2)); // Convert from sRGB to linear
        let Li = light.params0.x + light.params1.x; // intensity + flicker

        let start_d = light.params0.z;
        let end_d = max(light.params0.w, start_d + 1e-3);

        let L_vec = Lp - P;
        let dist = length(L_vec);
        let L = normalize(L_vec);
        let H = normalize(V + L);

        // Distance-based attenuation
        let dist2 = max(dot(L_vec, L_vec), 1e-6);
        let falloff = clamp((end_d - dist) / max(end_d - start_d, 1e-3), 0.0, 1.0);
        let attenuation = (Li * falloff) / dist2;

        // Ray-traced shadow
        let shadow = trace_shadow(P, L, dist);
        if (shadow < 0.01) { continue; }

        let radiance = Lc * attenuation * shadow;

        // Cook-Torrance BRDF
        let NdotL = max(dot(N, L), 0.0);
        let NdotV = max(dot(N, V), 0.0);

        if (NdotL > 0.0) {
            let NDF = distribution_ggx(N, H, mat.roughness);
            let G = geometry_smith(N, V, L, mat.roughness);
            let F = fresnel_schlick(max(dot(H, V), 0.0), F0);

            let numerator = NDF * G * F;
            let denominator = 4.0 * NdotV * NdotL + 1e-7;
            var specular = numerator / denominator;

            // Clamp specular to prevent explosion at grazing angles
            specular = min(specular, vec3<f32>(1.0));

            // Energy conservation
            let kS = F;
            let kD = (vec3<f32>(1.0) - kS) * (1.0 - mat.metallic);

            Lo += (kD * albedo / PI + specular) * radiance * NdotL;
        }
    }

    return Lo;
}

// ===== Main Compute Shader =====
@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;
    if (px >= U.fb_size.x || py >= U.fb_size.y) { return; }

    // Build camera ray
    let cam_uv = vec2<f32>(
        (f32(px) + 0.5) / f32(U.fb_size.x),
        (f32(py) + 0.5) / f32(U.fb_size.y)
    );
    let ray = cam_ray(cam_uv);
    var ro = ray.ro;
    let rd = normalize(ray.rd);

    // Accumulated color (front to back compositing)
    var accum_color = vec3<f32>(0.0);
    var accum_alpha = 0.0;
    var fog_distance = 0.0; // Track distance from camera for fog (set on first hit)
    var first_hit = true;

    // Sky color for background (convert from sRGB to linear)
    let sky_rgb_srgb = select(U.background.rgb, U.gp0.xyz, length(U.gp0.xyz) > 0.01);
    let sky_rgb = pow(sky_rgb_srgb, vec3<f32>(2.2));
    let ambient_strength = select(0.3, U.gp3.w, U.gp3.w > 0.0);

    // Trace through transparent layers
    for (var bounce: u32 = 0u; bounce < MAX_TRANSPARENCY_BOUNCES; bounce = bounce + 1u) {
        // First ray uses epsilon, continuation uses 0 to avoid self-intersection vs gaps
        let tmin = select(0.0, 0.001, bounce == 0u);
        let hit = sv_trace_grid(ro, rd, tmin, 1e6);

        if (!hit.hit) {
            // Hit sky - blend with accumulated alpha
            let sky_color = sky_rgb * (1.0 - accum_alpha);
            accum_color += sky_color;
            accum_alpha = 1.0;
            break;
        }

        // Track distance from camera (only first hit counts for fog)
        if (first_hit) {
            fog_distance = hit.t; // Simple: just use the ray parameter distance
            first_hit = false;
        }

        // Reconstruct hit information
        let tri = hit.tri;
        let i0 = indices3d.data[3u * tri + 0u];
        let i1 = indices3d.data[3u * tri + 1u];
        let i2 = indices3d.data[3u * tri + 2u];

        let v0 = verts3d.data[i0];
        let v1 = verts3d.data[i1];
        let v2 = verts3d.data[i2];

        // Barycentric interpolation
        let w = 1.0 - hit.u - hit.v;

        // Interpolate smooth normal
        var N = normalize(v0.normal * w + v1.normal * hit.u + v2.normal * hit.v);

        // Hit position
        let P = ro + rd * hit.t;

        // Sample albedo and material
        var albedo = sv_tri_sample_albedo(i0, i1, i2, hit.u, hit.v);
        // Convert user-defined sRGB colors to linear space for PBR calculations
        albedo = vec4<f32>(pow(albedo.rgb, vec3<f32>(2.2)), albedo.a);

        let mat_data = sv_tri_sample_rmoe(i0, i1, i2, hit.u, hit.v);
        let mat = unpack_material(mat_data);

        // Apply bump mapping
        if (BUMP_STRENGTH > 0.0 && length(mat.normal) > 0.1) {
            let TBN = sv_tri_tbn(v0.pos, v1.pos, v2.pos, v0.uv, v1.uv, v2.uv);
            let N_ts = mat.normal;
            let N_ws = normalize(TBN * N_ts);
            N = normalize(mix(N, N_ws, BUMP_STRENGTH));
        }

        // Two-sided lighting
        if (dot(N, rd) > 0.0) { N = -N; }

        let V = -rd;

        // Compute ambient occlusion
        let ao = compute_ao(P, N, P + vec3<f32>(f32(px), f32(py), f32(bounce)));

        // PBR direct lighting
        let direct = pbr_lighting(P, N, V, albedo.rgb, mat);

        // Ambient contribution
        let has_ambient_color = length(U.gp3.xyz) > 0.01;
        let ambient_color_srgb = select(vec3<f32>(0.05), U.gp3.xyz, has_ambient_color);
        let ambient_color = pow(ambient_color_srgb, vec3<f32>(2.2)); // Convert from sRGB to linear

        // Sky contribution: combine orientation and occlusion
        // How much the surface faces upward (0.0 = horizontal, 1.0 = straight up)
        let sky_factor = max(dot(N, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
        // Ray trace in reflection direction to check occlusion
        let sky_dir = reflect(rd, N);
        // Only trace if reflection actually points upward (sky is above)
        let sky_dir_up = max(dot(sky_dir, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
        let sky_visibility = select(0.0, trace_shadow(P, sky_dir, 1e6), sky_dir_up > 0.0);
        // Combine: orientation determines amount, ray trace determines visibility
        let sky_contribution = sky_rgb * sky_factor * sky_visibility;

        // Combine ambient (uniform) and sky (directional based on upward facing)
        let ambient = (ambient_color * ambient_strength + sky_contribution) * albedo.rgb * ao;

        // Emissive contribution (self-illumination, multiplied by 2.0 for visibility)
        let emissive = albedo.rgb * mat.emissive * 2.0;

        // Combine lighting for this layer
        var layer_color = direct + ambient + emissive;

        // Calculate layer opacity (from material and texture alpha)
        let layer_opacity = albedo.a * mat.opacity;

        // Handle opaque vs transparent surfaces differently
        if (layer_opacity >= 0.99) {
            // Fully opaque - blend any previous transparent layers and use this color
            accum_color += layer_color * (1.0 - accum_alpha);
            accum_alpha = 1.0;
            break;
        } else {
            // Transparent - front-to-back alpha compositing
            accum_color += layer_color * layer_opacity * (1.0 - accum_alpha);
            accum_alpha += layer_opacity * (1.0 - accum_alpha);

            // Check if we've accumulated enough opacity to stop
            if (accum_alpha >= 0.99) {
                accum_alpha = 1.0;
                break;
            }

            // Continue ray from just past this surface
            ro = P + rd * 0.001;
        }
    }

    // Apply fog based on distance traveled
    var final_color = accum_color;

    let fog_density = U.gp4.w;
    if (fog_density > 0.0) {
        // Exponential squared fog: fog_amount = density * distance²
        let fog_amount = fog_density * fog_distance * fog_distance;
        let fog_factor = clamp(exp(-fog_amount), 0.0, 1.0);
        let fog_color_srgb = U.gp4.xyz;
        let fog_color = pow(fog_color_srgb, vec3<f32>(2.2)); // Convert to linear

        // Mix between scene color and fog color based on fog factor
        // fog_factor = 1.0 means no fog (close), 0.0 means full fog (far)
        final_color = mix(fog_color, final_color, fog_factor);
    }

    // Apply tone mapping and gamma to accumulated color
    final_color = final_color / (final_color + vec3<f32>(1.0));
    final_color = pow(final_color, vec3<f32>(1.0 / 2.2));

    sv_write(px, py, vec4<f32>(final_color, accum_alpha));
}
