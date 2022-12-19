// Vertex shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>
}

@vertex
fn vs_main() -> vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0); // TODO: Fix
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
    return textureSample(grid_texture, grid_sampler, coord_in.xy); // Test with this shader
}