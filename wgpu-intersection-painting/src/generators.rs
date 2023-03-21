use core::fmt;
use std::path::PathBuf;

use crate::{RawImage, args, save_raw_image};

// Commands
pub fn generate_and_save_stencil(width: u32, height: u32, out_path: PathBuf, generator: args::Generator){
    let buffer = generate_stencil(width, height, &generator);
    save_raw_image(buffer, out_path);
}

pub fn generate_stencil(width: u32, height: u32, generator: &args::Generator) -> RawImage{
    RawImage{
        skip: 3,
        width,
        height,
        data: match generator{
            args::Generator::SquareGrid(args::SquareGridCommand{side_length: s}) => generate_square_grid(width, height, *s, 0),
            args::Generator::CircleGrid(args::CircleGridCommand{radius: r}) => generate_circle_grid(width, height, *r),
            args::Generator::CrossGrid(args::CrossGridCommand{cross_intersection_width}) => generate_cross_grid(width, height, *cross_intersection_width)
        },
        has_alpha: false
    }
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

        fn pair_from_floats(x: f32, y: f32) -> UintFloatPositionPair{
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

        let mut cur_pos = pair_from_floats(f_radius, 0 as f32);        // Starting at the top

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
                cur_pos = pair_from_floats(cur_pos.f_x + 1.0, cur_pos.f_y)
            }
            else if right_square_dist + (cur_pos.f_y - f_radius + 1.0).powi(2) <= f_r_2{
                cur_pos = pair_from_floats(cur_pos.f_x + 1.0, cur_pos.f_y + 1.0)
            }
            else{
                cur_pos = pair_from_floats(cur_pos.f_x, cur_pos.f_y + 1.0)
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
    let circle_mask = repeat_stencil_to_mask(circle, width, height);

    mask_container(circle_mask, (0, 0, 0), &mut squares).expect("Error masking");
    return squares
}

fn generate_cross_grid(width: u32, height: u32, cross_intersection_width: u32) -> Vec<u8>{
    let mut container = vec![0 as u8; (width as usize) * (height as usize) * 3];
    let grid_width = num::Integer::div_ceil(&width, &cross_intersection_width);
    let grid_height = num::Integer::div_ceil(&height, &cross_intersection_width);

    let cell_in_grid = |cell_x: i32, cell_y: i32|{
        return (cell_x >= 0 && (cell_x as u32) < grid_width) && (cell_y >= 0 && (cell_y as u32) < grid_height);
    };

    let mut fill_cell = |cell_x: i32, cell_y: i32, index: u32| {
        if !cell_in_grid(cell_x, cell_y){
            return;
        }

        let u_cell_x = cell_x as u32;
        let u_cell_y = cell_y as u32;

        if u_cell_x >= grid_width || u_cell_y >= grid_height{
            return;
        }

        let start_x = u_cell_x * cross_intersection_width;
        let start_y = u_cell_y * cross_intersection_width;

        let index_pixels = segment_index_to_rgb(index);

        for x in 0..cross_intersection_width{
            for y in 0..cross_intersection_width{
                let cur_x = start_x + x;
                if cur_x >= width{
                    continue;
                }

                let cur_y = start_y + y;
                if cur_y >= height{
                    continue;
                }

                container[((cur_x + width * cur_y) * 3) as usize] = index_pixels.0;
                container[((cur_x + width * cur_y) * 3 + 1) as usize] = index_pixels.1;
                container[((cur_x + width * cur_y) * 3 + 2) as usize] = index_pixels.2;
            }
        }
    };

    let mut fill_cross = |cell_x: i32, cell_y: i32, index: u32|{
        fill_cell(cell_x - 1, cell_y, index);
        fill_cell(cell_x + 1, cell_y, index);
        fill_cell(cell_x, cell_y -1, index);
        fill_cell(cell_x, cell_y + 1, index);
        fill_cell(cell_x, cell_y, index);
    };

    let cross_touching_grid = |cell_x: i32, cell_y: i32|{
        return cell_in_grid(cell_x - 1, cell_y) || cell_in_grid(cell_x + 1, cell_y) || cell_in_grid(cell_x, cell_y - 1) || cell_in_grid(cell_x, cell_y + 1) || cell_in_grid(cell_x, cell_y);
    };

    let mut fill_diag = |start_cell_x: i32, start_cell_y: i32, start_index: u32|{
        // Assumes that the starting position is in the grid.
        let mut index = start_index;
        let mut cell_x = start_cell_x;
        let mut cell_y = start_cell_y;


        while cross_touching_grid(cell_x, cell_y){
            fill_cross(cell_x, cell_y, index);
            index += 1;
            cell_x += 2;
            cell_y += 1;
        }

        return index;
    };

    // Let's start with the top row:
    let mut index = 0;
    let mut cell_x = 0;
    let mut cell_y = 0;
    
    loop{
        index = fill_diag(cell_x, cell_y, index);

        if cell_in_grid(cell_x, cell_y){
            cell_x += 3;
            cell_y -= 1;
        }
        else if cell_in_grid(cell_x, cell_y + 1){
            cell_x += 2;
            cell_y += 1;
        }
        else{
            break;
        }
    }

    // Now let's loop through the left column:
    cell_x = -1;
    cell_y = 2;
    loop{
        index = fill_diag(cell_x, cell_y, index);

        if cell_in_grid(cell_x, cell_y){
            cell_x -= 1;
            cell_y += 2;
        }
        else if cell_in_grid(cell_x + 1, cell_y){
            cell_x += 1;
            cell_y += 3;
        }
        else{
            break;
        }
    }

    return container;
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