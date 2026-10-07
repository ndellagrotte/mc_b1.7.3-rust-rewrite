// The GL state of the entity renderers' Tessellator draws
// (crates/mc-client/src/render/material.rs, project-owned): the vertex colour
// (the Tessellator's colour or glColor, already lit by GL's fixed-function
// lighting on the CPU, render/gl.rs) times the bound texture, GL's alpha test
// and GL's fog, all in the Java's colour values, converted to linear only for
// an sRGB target at the end, as gl_material.wgsl does. Textures are the
// shared cache's sRGB images: samples are re-encoded to the PNG's values.

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::view

struct EntityGlParams {
    // GL_FOG_COLOR
    fog_color: vec4<f32>,
    // x: 0 no fog, 1 GL_LINEAR, 2 GL_EXP; y: start; z: end; w: density
    fog: vec4<f32>,
    // glAlphaFunc(GL_GREATER, alpha_ref); negative: GL_ALPHA_TEST off
    alpha_ref: f32,
    // GL_TEXTURE_2D on
    textured: u32,
    // the bound image is sRGB: re-encode samples to the PNG's values
    texture_srgb: u32,
    // unused (alignment)
    pad: u32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: EntityGlParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gl_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gl_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
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
    var c = clamp(in.color, vec4<f32>(0.0), vec4<f32>(1.0));
    if params.textured != 0u {
        var t = textureSample(gl_texture, gl_sampler, in.uv);
        if params.texture_srgb != 0u {
            t = vec4<f32>(linear_to_srgb(t.rgb), t.a);
        }
        c = c * t;
    }
    if params.alpha_ref >= 0.0 && !(c.a > params.alpha_ref) {
        discard;
    }
    if params.fog.x > 0.5 {
        // GL_EYE_RADIAL_NV (EntityRenderer.java:907-909): the eye distance.
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
    return c;
#else ifdef GL_BLEND_ADDITIVE
    // glBlendFunc(GL_SRC_ALPHA, GL_ONE): exact over black (gl_material.rs).
    return vec4<f32>(srgb_to_linear(c.rgb * c.a) / max(c.a, 1.0e-6), c.a);
#else
    return vec4<f32>(srgb_to_linear(c.rgb), c.a);
#endif
}
