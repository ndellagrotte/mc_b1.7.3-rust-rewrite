// The GL state entity models are drawn with (crates/mc-client/src/model/
// material.rs, project-owned): glColor times GL's fixed-function lighting as
// RenderHelper.enableStandardItemLighting sets it up (two directional lights,
// diffuse 0.6, scene ambient 0.4, GL_COLOR_MATERIAL on ambient and diffuse),
// times the texture (with an optional GL_TEXTURE matrix translation), GL's
// alpha test and GL's fog; all in the Java's colour values, converted to
// linear only for an sRGB target at the end, as gl_material.wgsl does.

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_normal_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::view

struct ModelParams {
    // glColor4f (the entity brightness, a pass colour)
    color: vec4<f32>,
    // GL_FOG_COLOR
    fog_color: vec4<f32>,
    // x: 0 no fog, 1 GL_LINEAR, 2 GL_EXP; y: start; z: end; w: density
    fog: vec4<f32>,
    // GL_LIGHT0 / GL_LIGHT1 direction (world space, towards the light)
    light0: vec4<f32>,
    light1: vec4<f32>,
    // xy: the GL_TEXTURE matrix translation
    uv_offset: vec4<f32>,
    // glAlphaFunc(GL_GREATER, alpha_ref); negative: off
    alpha_ref: f32,
    // GL_TEXTURE_2D on
    textured: u32,
    // GL_LIGHTING on
    lighting: u32,
    // the bound image is sRGB (the shared texture cache): re-encode samples
    texture_srgb: u32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: ModelParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var model_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var model_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    // the lit glColor (GL lights per vertex; a face has one normal)
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
    out.uv = v.uv + params.uv_offset.xy;
    var c = params.color;
    if params.lighting != 0u {
        // GL_RESCALE_NORMAL: the transformed normal back to unit length.
        let n = mesh_normal_local_to_world(v.normal, v.instance_index);
        let d = max(dot(n, params.light0.xyz), 0.0) + max(dot(n, params.light1.xyz), 0.0);
        // emission 0 + scene ambient 0.4 * c + sum(diffuse 0.6 * max(n.L, 0) * c),
        // clamped; alpha is the diffuse material's.
        let rgb = clamp(c.rgb * 0.4 + c.rgb * 0.6 * d, vec3<f32>(0.0), vec3<f32>(1.0));
        c = vec4<f32>(rgb, clamp(c.a, 0.0, 1.0));
    }
    out.color = c;
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
        var t = textureSample(model_texture, model_sampler, in.uv);
        if params.texture_srgb != 0u {
            // The hardware decoded the sRGB image; back to the PNG's bytes / 255.
            t = vec4<f32>(linear_to_srgb(t.rgb), t.a);
        }
        c = c * t;
    }
    if params.alpha_ref >= 0.0 && !(c.a > params.alpha_ref) {
        discard;
    }
    if params.fog.x > 0.5 {
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
    return vec4<f32>(srgb_to_linear(c.rgb * c.a) / max(c.a, 1.0e-6), c.a);
#else
    return vec4<f32>(srgb_to_linear(c.rgb), c.a);
#endif
}
