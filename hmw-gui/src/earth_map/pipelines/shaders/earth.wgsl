struct Uniforms {
    mvp: mat4x4<f32>,
    dark_mode: u32,
}


struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) model_normal: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;


struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) model_normal: vec3f,
    @location(2) world_normal: vec3f,
};

@vertex
fn vs_main( 
    vertex: Vertex,
    @builtin(vertex_index) vertex_index : u32,
) -> VSOutput {
    var pos = vec4f(
        vertex.position.xyz,
        1.
    );

    var vs_out: VSOutput;
    vs_out.position = uniforms.mvp * pos;
    vs_out.model_normal = vertex.model_normal;
    vs_out.world_normal = normalize((uniforms.mvp * vec4f(vertex.model_normal, 0.)).xyz);
    return vs_out;
}

@group(0)
@binding(1)
var r_texture: texture_cube<f32>;
@group(0)
@binding(2)
var r_sampler: sampler;

@fragment
fn fs_main(in: VSOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.model_normal);
    let sample = textureSample(r_texture, r_sampler, normal);
    let base = sample.rgb;
    let world_normal = normalize(in.world_normal);
    let fc = fresnel(4.0, world_normal);

    if uniforms.dark_mode != 0u {
        let rgb = clamp(
            base * 0.5 - fc,
            vec3f(0.0),
            vec3f(1.0),
        );

        return vec4f(rgb, sample.a);
    }

    let rgb = clamp(base + fc, vec3f(0.0), vec3f(1.0));

    return vec4f(rgb, sample.a);
}


fn fresnel(amount: f32, normal: vec3f) -> f32 {
    let d = clamp(dot(normal, vec3f(0., 0., -1.)), 0.0, 1.0);

    return pow(1.0 - d, amount);
}
