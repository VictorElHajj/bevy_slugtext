#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}


// ===================================================
// Reference vertex shader for the Slug algorithm ported to WGSL
// ===================================================

struct SlugUnpackResult {
    vbnd: vec4<f32>,
    vgly: vec4<i32>,
}

fn SlugUnpack(tex: vec4<f32>, bnd: vec4<f32>) -> SlugUnpackResult {
    let g = vec2<u32>(bitcast<u32>(tex.z), bitcast<u32>(tex.w));
    let vgly = vec4<i32>(
        i32(g.x & 0xFFFFu),
        i32(g.x >> 16u),
        i32(g.y & 0xFFFFu),
        i32(g.y >> 16u)
    );
    return SlugUnpackResult(bnd, vgly);
}

struct SlugDilateResult {
    texcoord: vec2<f32>,
    vpos: vec2<f32>,
}

fn SlugDilate(pos: vec4<f32>, tex: vec4<f32>, jac: vec4<f32>, m0: vec4<f32>, m1: vec4<f32>, m3: vec4<f32>, dim: vec2<f32>) -> SlugDilateResult {
    let n = normalize(pos.zw);
    let s = dot(m3.xy, pos.xy) + m3.w;
    let t = dot(m3.xy, n);

    let u = (s * dot(m0.xy, n) - t * (dot(m0.xy, pos.xy) + m0.w)) * dim.x;
    let v = (s * dot(m1.xy, n) - t * (dot(m1.xy, pos.xy) + m1.w)) * dim.y;

    let s2 = s * s;
    let st = s * t;
    let uv = u * u + v * v;
    let d = pos.zw * (s2 * (st + sqrt(uv)) / (uv - st * st));

    let vpos = pos.xy + d;
    let texcoord = vec2<f32>(tex.x + dot(d, jac.xy), tex.y + dot(d, jac.zw));
    return SlugDilateResult(texcoord, vpos);
}

@group(#{MATERIAL_BIND_GROUP}) @binding(104) var<uniform> billboard: u32;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(8) pos: vec4<f32>,
    @location(9) tex: vec4<f32>,
    @location(10) jac: vec4<f32>,
    @location(11) bnd: vec4<f32>,
};

struct VertexStruct {
    @builtin(position) position: vec4<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) @interpolate(flat) banding: vec4<f32>,
    @location(4) @interpolate(flat) glyph: vec4<i32>,
};

@vertex
fn main(attrib: VertexInput) -> VertexStruct {
    var vresult: VertexStruct;

    let slug_viewport = view.viewport.zw;
    let model_matrix = mesh_functions::get_world_from_local(attrib.instance_index);

    if (billboard == 1u) {
        // Camera-facing, constant-screen-size billboard.
        // Only the model origin (translation) is projected; the glyph offset (already in
        // pixel units) is added in screen space and scaled by clip_origin.w so it stays a
        // constant pixel size after the perspective divide.
        let world_origin = model_matrix[3];
        let clip_origin = view.clip_from_world * world_origin;

        // Glyph->screen is a uniform scale here, so Slug's screen-space dilation collapses
        // to a constant ~half-pixel offset along the corner normal.
        let n = normalize(attrib.pos.zw);
        let d = n * 0.25;
        let vpos = attrib.pos.xy + d;
        vresult.texcoord = vec2<f32>(
            attrib.tex.x + dot(d, attrib.jac.xy),
            attrib.tex.y + dot(d, attrib.jac.zw)
        );

        // viewport.zw = (width, height); use each axis independently to avoid aspect stretch.
        let ndc = vpos * 2.0 / slug_viewport;
        vresult.position = vec4<f32>(
            clip_origin.x + ndc.x * clip_origin.w,
            clip_origin.y + ndc.y * clip_origin.w,
            clip_origin.z,
            clip_origin.w
        );
    } else {
        let sm = transpose(view.clip_from_world * model_matrix);

        let m0 = sm[0];
        let m1 = sm[1];
        let m2 = sm[2];
        let m3 = sm[3];

        let dilateResult = SlugDilate(attrib.pos, attrib.tex, attrib.jac, m0, m1, m3, slug_viewport);
        vresult.texcoord = dilateResult.texcoord;
        let p = dilateResult.vpos;

        vresult.position = vec4<f32>(
            p.x * m0.x + p.y * m0.y + m0.w,
            p.x * m1.x + p.y * m1.y + m1.w,
            p.x * m2.x + p.y * m2.y + m2.w,
            p.x * m3.x + p.y * m3.y + m3.w
        );
    }

    let unpackResult = SlugUnpack(attrib.tex, attrib.bnd);
    vresult.banding = unpackResult.vbnd;
    vresult.glyph = unpackResult.vgly;
    return vresult;
}