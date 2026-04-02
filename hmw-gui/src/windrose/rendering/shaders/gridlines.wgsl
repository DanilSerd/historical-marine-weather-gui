struct Uniforms {
    view: mat4x4<f32>,
    segments_per_gridline: u32,
    gridlines: u32,
    gridline_thickness: f32,
    _unused: u32,
    gridline_color: vec4f,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VSOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

const PI2: f32 = 6.28318;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index : u32,
    @builtin(instance_index) instance_index: u32
) -> VSOutput {
    // instance index is the gridline index. 0 being closest to the center of the rose.
    // Each gridline segment is a quad with 2 triangles.

    let current_gridline_distance = f32(instance_index + 1u) * 1f / f32(uniforms.gridlines);
    let current_vertex_segment = vertex_index / 6u;

    let angle_per_segment = PI2 / f32(uniforms.segments_per_gridline);
    let segment_start_angle = f32(current_vertex_segment) * angle_per_segment;
    let segment_end_angle = f32(current_vertex_segment + 1u) * angle_per_segment;

    var final_angle: f32;
    var radius: f32;

    switch vertex_index % 6u {
        case default, 0u, 3u, 5u {
            if current_gridline_distance >= 0.99 {
                radius = 1. - uniforms.gridline_thickness;
            } else {
                radius = current_gridline_distance - uniforms.gridline_thickness / 2.0;
            }
        }
        case 1u, 4u, 2u {
            if current_gridline_distance >= 0.99 {
                radius = 1.;
            } else {
                radius = current_gridline_distance + uniforms.gridline_thickness / 2.0;
            }
        }
    } 

    switch vertex_index % 6u {
        case default, 0u, 1u, 3u {
            final_angle = segment_start_angle;
        }
        case 2u, 4u, 5u {
            final_angle = segment_end_angle;
        }
    }


    let pos = vec4f(
        cos(final_angle) * radius,
        sin(final_angle) * radius,
        0.,
        1.
    );

    var vs_out: VSOutput;
    vs_out.position = uniforms.view * pos;
    vs_out.color = uniforms.gridline_color;

    return vs_out;
}

@fragment
fn fs_main(in: VSOutput) -> @location(0) vec4<f32> {
    return in.color;
}
