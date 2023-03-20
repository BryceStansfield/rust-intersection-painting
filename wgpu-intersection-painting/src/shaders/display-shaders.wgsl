// Vertex shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// Fragement Shader, assumes that we're just shading a rectangle at distance z=1, w=1 from the camera.
// Intersection result array
@group(0) @binding(0)
var grid_texture: texture_2d<f32>;
@group(0) @binding(1)
var grid_sampler: sampler;
@group(0) @binding(2)
readonly var<storage, read> occupied: array<vec4<f32>>;

@fragment
fn fs_main(@builtin(position) coord_in: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 1.0);//return textureSample(grid_texture, grid_sampler, coord_in.xy); // Test with this shader
}