// The terrain's fixed-function GL state (crates/mc-client/src/terrain/material.rs,
// project-owned): the Tessellator's vertex colour (four bytes) times glColor
// times terrain.png, GL's alpha test (GL_GREATER), GL's fog by eye distance,
// all on the Java's colour values; converted to linear only for an sRGB
// target at the end, as gl_material.wgsl does. The terrain atlas of WP-3.2
// is an sRGB texture, so a sample is re-encoded to the stored byte / 255
// first (TERRAIN_SRGB_TEXTURE). The crack overlay (TERRAIN_CRACK) is
// blended with glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR): GL writes 2 * s * d.

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::view

struct GlParams {
    color: vec4<f32>,
    fog_color: vec4<f32>,
    // x: 0 no fog, 1 GL_LINEAR, 2 GL_EXP; y: start; z: end; w: density
    fog: vec4<f32>,
    // glAlphaFunc(GL_GREATER, alpha_ref); negative: off
    alpha_ref: f32,
    textured: u32,
    line_width: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: GlParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var terrain_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var terrain_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

// The texture coordinates and colours are interpolated at the centroid: with
// the camera's MSAA a pixel on a quad's edge is shaded at its centre, which
// can lie outside the quad, and a coordinate extrapolated there samples the
// neighbouring tile of the atlas (a dark seam between blocks). Vanilla draws
// without multisampling, where the centre is always inside.
struct VertexOutput {
    @invariant @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) @interpolate(perspective, centroid) uv: vec2<f32>,
    @location(2) @interpolate(perspective, centroid) color: vec4<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world = mesh_position_local_to_world(
        get_world_from_local(v.instance_index),
        vec4<f32>(v.position, 1.0),
    );
    out.world_position = world.xyz;
    out.clip_position = position_world_to_clip(world.xyz);
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var c = clamp(in.color * params.color, vec4<f32>(0.0), vec4<f32>(1.0));
    if params.textured != 0u {
        var t = textureSample(terrain_texture, terrain_sampler, in.uv);
#ifdef TERRAIN_SRGB_TEXTURE
        t = vec4<f32>(linear_to_srgb(t.rgb), t.a);
#endif
        c = c * t;
    }
    if params.alpha_ref >= 0.0 && !(c.a > params.alpha_ref) {
        discard;
    }
    if params.fog.x > 0.5 {
        // GL_EYE_RADIAL_NV (EntityRenderer.java:907-909): distance from the eye.
        let d = distance(in.world_position, view.world_position);
        var f: f32;
        if params.fog.x < 1.5 {
            f = (params.fog.z - d) / (params.fog.z - params.fog.y);
        } else {
            f = exp(-params.fog.w * d);
        }
        f = clamp(f, 0.0, 1.0);
        c = vec4<f32>(mix(params.fog_color.rgb, c.rgb, f), c.a);
    }
#ifdef GL_TARGET_UNORM
    // The main target stores the values as they are: GL's bytes.
    return c;
#else ifdef TERRAIN_CRACK
    // GL writes 2 * s * d (bytes); the hardware computes 2 * out * d in
    // linear space. With sRGB as a 2.2 power, 2 * s * D needs
    // out = (2 s)^2.2 / 2 (neutral at s = 0.5, as in GL).
    return vec4<f32>(pow(2.0 * c.rgb, vec3<f32>(2.2)) * 0.5, c.a);
#else
    return vec4<f32>(srgb_to_linear(c.rgb), c.a);
#endif
}
