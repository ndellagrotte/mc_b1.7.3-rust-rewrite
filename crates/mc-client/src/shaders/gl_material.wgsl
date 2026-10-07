// The fixed-function GL state the sky, clouds and weather are drawn with
// (crates/mc-client/src/gl_material.rs, project-owned): vertex colour times
// glColor times an optional texture, GL's alpha test, GL's linear or
// exponential fog by eye distance, all in the Java's (non-linear) colour
// values, which are converted to linear only for the sRGB target at the end
// (additive draws pre-multiplied, see gl_material.rs "Colours"); and
// glLineWidth for wide lines (GL_LINE_WIDTH).

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
    // glLineWidth, in pixels (GL_LINE_WIDTH meshes)
    line_width: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: GlParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gl_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gl_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
#ifdef GL_LINE_WIDTH
    // The segment's other end (local space); uv.x is the side, -1 or 1.
    @location(3) other: vec3<f32>,
#endif
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
#ifdef GL_LINE_WIDTH
    // An aliased wide line as GL rasterises it: the projected segment
    // widened by line_width pixels along its minor screen axis (a column for
    // an x-major line, a row for a y-major one). The ends are first clipped
    // to w >= near so the screen direction is that of the visible part.
    let far_end = mesh_position_local_to_world(
        get_world_from_local(v.instance_index),
        vec4<f32>(v.other, 1.0),
    );
    var c1 = position_world_to_clip(far_end.xyz);
    let w_min = 1.0e-3;
    if clip.w < w_min && c1.w < w_min {
        // Entirely behind the eye: outside the frustum.
        clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    } else {
        if clip.w < w_min {
            clip = mix(clip, c1, (w_min - clip.w) / (c1.w - clip.w));
        }
        if c1.w < w_min {
            c1 = mix(c1, clip, (w_min - c1.w) / (clip.w - c1.w));
        }
        let vp = view.viewport.zw;
        let d = (c1.xy / c1.w - clip.xy / clip.w) * vp;
        // Half the width, in NDC (2 NDC units span the viewport).
        var off: vec2<f32>;
        if abs(d.x) >= abs(d.y) {
            off = vec2<f32>(0.0, params.line_width / vp.y);
        } else {
            off = vec2<f32>(params.line_width / vp.x, 0.0);
        }
        clip = vec4<f32>(clip.xy + off * v.uv.x * clip.w, clip.zw);
    }
#endif
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
#ifdef GL_TARGET_UNORM
    // The main target stores the values as they are (the camera's
    // CompositingSpace::Srgb): GL's bytes, blended as GL blends them.
    return c;
#else ifdef GL_BLEND_ADDITIVE
    // glBlendFunc(GL_SRC_ALPHA, GL_ONE) adds the bytes c·a; the hardware
    // multiplies the returned colour by a and adds it in linear space, so
    // return linear(c·a) / a: exact over a black destination (gl_material.rs,
    // "Colours"). linear(x) <= x, so this stays within 0..1.
    return vec4<f32>(srgb_to_linear(c.rgb * c.a) / max(c.a, 1.0e-6), c.a);
#else
    return vec4<f32>(srgb_to_linear(c.rgb), c.a);
#endif
}
