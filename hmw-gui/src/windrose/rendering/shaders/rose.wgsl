struct Uniforms {
    view: mat4x4<f32>,
    subdivisions_per_sector: u32,
    highlight_segment: u32,
}


struct RoseSector {
    color: vec4f,
    inner: f32,
    outer: f32,
    sweep_start_angle: f32,
    sweep_end_angle: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> sectors: array<RoseSector>;

// Fudge factor to prevent artifacts where the triangles don't meet.
const FUDGE: f32 = 0.0001;

struct VSOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

@vertex
fn vs_main( 
    @builtin(vertex_index) vertex_index : u32,
    @builtin(instance_index) instance_index: u32
) -> VSOutput {
    var my_sector = sectors[instance_index];
    if instance_index == uniforms.highlight_segment {
        my_sector.color *= vec4f(0.5, 0.5, 0.5, 1.);
    }

    let angle_portion_per_section = (my_sector.sweep_end_angle - my_sector.sweep_start_angle) / f32(uniforms.subdivisions_per_sector);
    let angle = my_sector.sweep_start_angle + (f32(vertex_index / 6u) * angle_portion_per_section); 

    var final_angle: f32;
    var radius: f32;

    switch vertex_index % 6u {
        case default, 0u, 2u, 3u {
            radius = my_sector.inner - FUDGE;
        }
        case 1u, 4u, 5u {
            radius = my_sector.outer;
        }
    } 

    switch vertex_index % 6u {
        case default, 0u, 4u, 1u {
            final_angle = angle;
        }
        case 2u, 3u, 5u {
            final_angle = angle + angle_portion_per_section;
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
    vs_out.color = my_sector.color;

    return vs_out;

}

@fragment
fn fs_main(in: VSOutput) -> @location(0) vec4<f32> {
    return in.color;
}