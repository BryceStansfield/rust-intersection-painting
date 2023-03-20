use core::fmt;
use std::path::PathBuf;
use num::Integer;

use crate::{RawImage, args, save_buffer_as_image};

// Commands
pub fn generate_and_save_stencil(width: u32, height: u32, out_path: PathBuf, generator: args::Generator){
    let mut buffer = generate_stencil(width, height, generator);
    save_buffer_as_image(buffer, width, height, out_path);
}

pub fn generate_stencil(width: u32, height: u32, generator: args::Generator) -> Vec<u8>{
    match generator{
        args::Generator::SquareGrid(args::SquareGridCommand{side_length: s}) => generate_square_grid(width, height, s, 0),
        args::Generator::CircleGrid(args::CircleGridCommand{radius: r}) => generate_circle_grid(width, height, r),
    }
}

pub (crate) fn stencil_to_raw_image(stencil: &Vec<u8>, width: u32, height: u32) -> RawImage{
    RawImage { skip: 3, width: width, height: height, data: stencil, has_alpha: false }
}

// Utility Functions
fn segment_index_to_rgb(ind: u32) -> (u8, u8, u8){
    return ((ind % 256) as u8, ((ind / 256) % 256) as u8, (ind / 65536) as u8);
}

// Generators
fn generate_square_grid(width: u32, height: u32, side_length: u32, start_at: u32) -> Vec<u8>{
    let mut container = vec![0 as u8; (width as usize)*(height as usize)*3];

    let squares_per_row = ((width as f32)/(side_length as f32)).ceil() as u32;

    let mut container_ind: u32 = 0;

    // I think a bounds check should be faster than a division. Need to check later.
    for y in 0..height{
        for x in 0..width{
            let square_x = x/side_length;
            let square_y = y/side_length;

            let segment_index = square_x + square_y * squares_per_row + start_at;
            let segment_rgb = segment_index_to_rgb(segment_index);

            container[container_ind as usize] = segment_rgb.0;
            container[(container_ind + 1) as usize] = segment_rgb.1;
            container[(container_ind + 2) as usize] = segment_rgb.2;
            container_ind += 3;
        }
    }

    return container;
}

struct BoolStencilPrinter<'a>{
    stencil: &'a Vec<Vec<bool>>
}

impl fmt::Display for BoolStencilPrinter<'_>{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in self.stencil{
            if let Err(x) = writeln!(f, "{}\n", line.iter().map(|b| if *b {'.'} else {','}).collect::<String>()) {
                return Err(x);
            }
        }
        return Ok(());
    }
}

fn generate_circle_grid(width: u32, height: u32, radius: u32) -> Vec<u8>{
    // First let's generate a circle of size circle_size in a temporary vector, then copy it repeatedly into our output.
    // Note: With this algorithm a circle of radius n actually takes up 2n+1 pixels in width.
    fn generate_circle_stencil(radius: u32) -> Vec<Vec<bool>>{
        let mut ret_val = vec![vec![false; (radius*2+1) as usize]; (radius*2+1) as usize];

        let f_radius = radius as f32;
        let f_r_2 = f_radius * f_radius;
        
        #[derive(Clone, Copy)]
        struct UintFloatPositionPair{
            u_x: u32,
            u_y: u32,
            f_x: f32,
            f_y: f32,
        }

        fn pairFromFloats(x: f32, y: f32) -> UintFloatPositionPair{
            UintFloatPositionPair { u_x: x as u32, u_y: y as u32, f_x: x, f_y: y }
        }

        // Turns out we don't need the horizontal path... TODO: Remove this and slightly refactor
        fn draw_line_between(start: u32, end: u32, x: u32, v: &mut Vec<Vec<bool>>) -> (){
            if start == end{
                v[start as usize][x as usize] = true;
                return;
            }

            let dir = if start < end {1} else {-1};
            let mut cur = start;

            v[start as usize][x as usize] = true;
            while cur != end{
                cur = cur.checked_add_signed(dir).expect("This shouldn't overflow...");
                v[cur as usize][x as usize] = true;
            }
        }

        let mut cur_pos = pairFromFloats(f_radius, 0 as f32);        // Starting at the top

        fn fill_circle_at(cur_pos: UintFloatPositionPair, radius: u32, v: &mut Vec<Vec<bool>>){
            draw_line_between(cur_pos.u_y, 2*radius - cur_pos.u_y, cur_pos.u_x, v);      // mid right
            draw_line_between(cur_pos.u_y, 2*radius - cur_pos.u_y, 2*radius - cur_pos.u_x, v);  // mid left
            draw_line_between(cur_pos.u_x, 2*radius - cur_pos.u_x, cur_pos.u_y, v);  // far left
            draw_line_between(cur_pos.u_x, 2*radius - cur_pos.u_x, 2*radius - cur_pos.u_y, v);  // far right
        }

        loop{
            // First, let's fill in the vertical lines from our existing position
            fill_circle_at(cur_pos, radius, &mut ret_val);

            // Now let's move counterclockwise.
            let right_square_dist = (cur_pos.f_x - f_radius + 1.0).powi(2);
            if right_square_dist + (cur_pos.f_y - f_radius).powi(2) <= f_r_2{
                cur_pos = pairFromFloats(cur_pos.f_x + 1.0, cur_pos.f_y)
            }
            else if right_square_dist + (cur_pos.f_y - f_radius + 1.0).powi(2) <= f_r_2{
                cur_pos = pairFromFloats(cur_pos.f_x + 1.0, cur_pos.f_y + 1.0)
            }
            else{
                cur_pos = pairFromFloats(cur_pos.f_x, cur_pos.f_y + 1.0)
            }

            if (cur_pos.u_x - radius) < cur_pos.u_y {        // 45deg angle
                //fill_circle_at(cur_pos, usize_radius, &mut ret_val);
                break;
            }
        }

        return ret_val;
    }

    let circle = generate_circle_stencil(radius);

    // Now let's copy this stencil all over the image.
    let mut squares = generate_square_grid(width, height, circle.len() as u32, 1);
    let mut circle_mask = repeat_stencil_to_mask(circle, width, height);

    mask_container(circle_mask, (0, 0, 0), &mut squares).expect("Error masking");
    return squares
}

fn generate_cross_grid(width: u32, height: u32, cross_intersection_width: u32){
    let mut container = vec![0 as u8; (width as usize) * (height as usize) * 3];
    /* let grid_width = width.div_ceil(cross_intersection_width); Not stabilized yet :'(
    let grid_height = height.div_ceil(cross_intersection_width); */

    let fill_cell = |cell_x: u32, cell_y: u32, index: u32| {
        if cell_x < 0 || cell_x >= grid_width{
            return;
        }
        if cell_y < 0 || cell_y >= grid_height{
            return;
        }

        let start_x = cell_x * cross_intersection_width;
        let start_y = cell_y * cross_intersection_width;

        let index_pixels = segment_index_to_rgb(index);

        let start_posn = start_x + width * start_y;

        for x in 0..cross_intersection_width{
            for y in 0..cross_intersection_width{
                let cur_x = start_x + x;
                if cur_x < 0 || cur_x >= width{
                    continue;
                }

                let cur_y = start_y + y;
                if cur_y < 0 || cur_y >= height{
                    continue;
                }


                container[((cur_x + width * cur_y) * 3) as usize] = index_pixels.0;
                container[((cur_x + width * cur_y) * 3 + 1) as usize] = index_pixels.1;
                container[((cur_x + width * cur_y) * 3 + 2) as usize] = index_pixels.2;
            }
        }
    };

    let fill_cross = |cell_x: u32, cell_y: u32, index: u32|{

    }

    let mut index = 0;

}


// Utility functions
#[derive(Debug)]
enum MaskingError{
    LengthMismatch
}

fn repeat_stencil_to_mask(stencil: Vec<Vec<bool>>, width: u32, height: u32) -> Vec<bool>{
    let mut mask = vec![false; (width as usize)*(height as usize)];

    let y_len = stencil.len() as u32;
    let x_len = stencil[0].len() as u32;
    
    for y in 0..height{
        for x in 0..width{
            mask[(x + y * width) as usize] = stencil[(y % y_len) as usize][(x % x_len) as usize];
        }
    }

    return mask;
}

fn mask_container(bool_mask: Vec<bool>, false_value: (u8,u8, u8), container: &mut Vec<u8>) -> Result<(), MaskingError>{
    // Masks `container` s.t. if bool_mask[i] == false, then pixel[i] = false_value;
    if container.len() != 3*bool_mask.len(){
        return Err(MaskingError::LengthMismatch);
    }

    for i in 0..bool_mask.len(){
        if !bool_mask[i]{
            container[3*i] = false_value.0;
            container[3*i + 1] = false_value.1;
            container[3*i + 2] = false_value.2;
        }
    }

    return Ok(());
}

fn shitty_div_ceil<N: Integer>(a: N, b: N) -> N{
    
}