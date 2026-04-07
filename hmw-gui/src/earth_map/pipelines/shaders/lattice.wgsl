struct Uniforms {
    vp: mat4x4<f32>,
    highlight_color: vec4f,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct Vertex {
    @location(0) position: vec3f,
    @location(1) color_intensity: f32,
}

struct VSOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

@vertex
fn vs_main( 
    vertex: Vertex,
) -> VSOutput {
    var vs_out: VSOutput;
    vs_out.position = uniforms.vp * vec4f(vertex.position, 1.0);
    let alpha = uniforms.highlight_color.w * vertex.color_intensity;
    vs_out.color = vec4f(uniforms.highlight_color.xyz * alpha, alpha);

    return vs_out;
}

@fragment
fn fs_main(in: VSOutput) -> @location(0) vec4<f32> {
    return in.color;
}
