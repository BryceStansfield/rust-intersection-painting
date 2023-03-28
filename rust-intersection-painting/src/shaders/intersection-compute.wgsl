// Grid texture we're sampling from
@group(0) @binding(0)
var grid_texture: texture_2d<f32>;
@group(0) @binding(1)
var line_texture: texture_2d<f32>;

@group(0) @binding(2)
var texture_sampler: sampler;

// Intersection result array
@group(0) @binding(3)
var<storage, read_write> occupied: array<vec4<f32>>;

@compute
@workgroup_size(1)
fn intersection_computer(@builtin(global_invocation_id) global_invocation_id: vec3<u32>){//, width: i32, height: i32) { TODO: Uniformity problems :'(
    var start_x: f32 = f32(global_invocation_id.x) / f32(1920);
    var start_y: f32 = f32(global_invocation_id.y) / f32(1080);
    //for (var x: f32 = start_x; x < start_x + width; x += 1){
    //    for (var y: f32 = start_y; y < start_y + height; y += 1){
    //        var line_texture_at_pt: vec4<f32> = textureSample(line_texture, s_diffuse, vec2<f32>(x, y));
    //        if (line_texture_at_pt.a != 0f){
    //            var grid_texture_at_pt: vec4<f32> = textureSample(grid_texture, s_diffuse, vec2<f32>(x, y));
    //            var grid_cell_index: i32 = i32(round(255*grid_texture_at_pt.r) + round(255*grid_texture_at_pt.b)*256 + round(255*grid_texture_at_pt.b)*65536);
    //            occupied[grid_cell_index] = line_texture_at_pt;     // TODO: Qn: How do we get rid of flickering?
    //        }
    //    }
    //}

    // There seem to be some color issues here... I'll try finishing this up and see if it displays properly.
    var line_texture_at_pt: vec4<f32> = textureSampleLevel(line_texture, texture_sampler, vec2<f32>(start_x, start_y), f32(0));
    var grid_texture_at_pt: vec4<f32> = textureSampleLevel(grid_texture, texture_sampler, vec2<f32>(start_x, start_y), f32(0));
    if (line_texture_at_pt.a != 0f){
        var grid_cell_index: i32 = i32(round(f32(255)*grid_texture_at_pt.r) + round(f32(255)*grid_texture_at_pt.g)*f32(256) + round(f32(255)*grid_texture_at_pt.b)*f32(65536));
        occupied[grid_cell_index] = line_texture_at_pt;     // TODO: Qn: How do we get rid of flickering?
    }
}