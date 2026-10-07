// The fixed-function GL state the sky, clouds and weather are drawn with
// (crates/mc-client/src/gl_material.rs, project-owned): vertex colour times
// glColor times an optional texture, GL's alpha test, GL's linear or
// exponential fog by eye distance, all in the Java's (non-linear) colour
// values, which are converted to linear only for the sRGB target at the end.

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::view

struct GlParams {
    // glColor4f
    color: vec4<f32>,
    // GL_FOG_COLOR
    fog_color: vec4<f32>,
    // x: 0 no fog, 1 GL_LINEAR, 2 GL_EXP; y: GL_FOG_START; z: GL_FOG_END; w: GL_FOG_DENSITY
    fog: vec4<f32>,
    // glAlphaFunc(GL_GREATER, alpha_ref); negative: GL_ALPHA_TEST off
    alpha_ref: f32,
    // GL_TEXTURE_2D on
    textured: u32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: GlParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gl_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gl_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @invariant @builtin(position) clip_position: vec4<f32>,
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
    var clip = position_world_to_clip(world.xyz);
#ifdef GL_SKY_FAR
    // The sky is drawn first with depth writes off in vanilla, so the world
    // covers it everywhere; here it is drawn after the opaque world, at the
    // far plane (reverse-Z: 0), so the depth test does the same.
    clip.z = 0.0;
#endif
    out.clip_position = clip;
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var c = clamp(in.color * params.color, vec4<f32>(0.0), vec4<f32>(1.0));
    if params.textured != 0u {
        c = c * textureSample(gl_texture, gl_sampler, in.uv);
    }
    if params.alpha_ref >= 0.0 && !(c.a > params.alpha_ref) {
        discard;
    }
    if params.fog.x > 0.5 {
        // GL_NV_fog_distance's GL_EYE_RADIAL_NV, which vanilla asks for
        // (EntityRenderer.java:907-909): the distance from the eye.
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
    return vec4<f32>(srgb_to_linear(c.rgb), c.a);
}
