struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var source: texture_multisampled_2d<f32>;
@group(0) @binding(1) var<uniform> view: mat4x4<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2f, 6>(
        vec2f(-1.0, -1.0),
        vec2f(1.0, -1.0),
        vec2f(-1.0, 1.0),
        vec2f(1.0, -1.0),
        vec2f(1.0, 1.0),
        vec2f(-1.0, 1.0)
    );
    
    var uv = array<vec2f, 6>(
        vec2f(0.0, 1.0),
        vec2f(1.0, 1.0),
        vec2f(0.0, 0.0),
        vec2f(1.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 0.0)
    );

    var output: VertexOutput;
    output.position = view * vec4f(pos[vertex_index], 0.0, 1.0);
    output.uv = uv[vertex_index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let tex_size = textureDimensions(source);
    let sample_count = textureNumSamples(source);
    
    // Convert UV to pixel coordinates with proper rounding
    let pixel_coord = vec2i(floor(input.uv * vec2f(tex_size) + 0.5));
    
    
    var color = vec4f(0.0);
    
    // Sample all samples and accumulate
    for(var i = 0u; i < sample_count; i++) {
        color += textureLoad(source, pixel_coord, i32(i));
    }
    
    // Average the samples
    return color / f32(sample_count);
} 