use core::fmt;
use std::path::PathBuf;

use itertools::Itertools;

use crate::{RawImage, args, save_raw_image, image_tools, stenciler::{rgb_to_index}};

pub const BYTES_PER_PIXEL: u32 = 4;         // Wgpu doesn't support 24 bit colours.

// Commands
pub fn generate_and_save_stencil(width: u32, height: u32, out_path: PathBuf, generator: args::Generator){
    let buffer = generate_stencil(width, height, &generator);
    save_raw_image(buffer, out_path);
}

pub fn generate_stencil(width: u32, height: u32, generator: &args::Generator) -> RawImage{
    RawImage{
        width,
        height,
        data: match generator{
            args::Generator::SquareGrid(args::SquareGridCommand{side_length: s}) => generate_square_grid(width, height, *s, 0),
            args::Generator::CircleGrid(args::CircleGridCommand{radius: r}) => generate_circle_grid(width, height, *r),
            args::Generator::ConcentricCircleGrid(args::ConcentricCircleGridCommand{radius: r}) => generate_concentric_circle_grid(width, height, *r),
            args::Generator::CrossGrid(args::CrossGridCommand{cross_intersection_width}) => generate_cross_grid(width, height, *cross_intersection_width),
            args::Generator::MaskGrid(args::MaskGridCommand{mask_folder}) => generate_from_masks(width, height, mask_folder),
            args::Generator::FloodFill(args::FloodFillCommand{mask_path}) => generate_fill_bucket(mask_path.to_owned())

        },
    }
}

// Utility Functions
fn segment_index_to_rgb(ind: u32) -> (u8, u8, u8){
    return ((ind % 256) as u8, ((ind / 256) % 256) as u8, (ind / 65536) as u8);
}

// Generators
fn generate_square_grid(width: u32, height: u32, side_length: u32, start_at: u32) -> Vec<u8>{
    let mut container = vec![0 as u8; (width as usize)*(height as usize)*BYTES_PER_PIXEL as usize];

    let squares_per_row = num::Integer::div_ceil(&width, &side_length);

    let mut container_ind: u32 = 0;

    // I think a bounds check should be faster than a division. Need to check later.
    for y in 0..height{
        for x in 0..width{
            let square_x = x/side_length;
            let square_y = y/side_length;

            let segment_index = square_x + square_y * squares_per_row + start_at;
            container_ind = fill_pixel_with_segindex(&mut container, container_ind, segment_index);
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
    let mut container = vec![0 as u8; (width as usize) * (height as usize) * BYTES_PER_PIXEL as usize];
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

                fill_pixel_with_segindex(&mut container, (cur_x + width * cur_y) * BYTES_PER_PIXEL, index);
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

fn generate_from_masks(width: u32, height: u32, mask_folder_path: &PathBuf) -> Vec<u8>{
    let mut masks: Vec<Vec<Vec<bool>>> = vec![];

    for (_, image) in image_tools::DynamicImageFolderIterator::new(mask_folder_path){
        match image{
            image::DynamicImage::ImageLuma8(greyscale_im) => {
                if !masks.is_empty() && (greyscale_im.width() as usize != masks[0][0].len() || greyscale_im.height() as usize != masks[0].len()){
                    panic!("All mask images must be the same size")
                }

                let mut bool_mask: Vec<Vec<bool>> = vec![vec![false; greyscale_im.width() as usize]; greyscale_im.height() as usize];

                for (x, y, pixel) in greyscale_im.enumerate_pixels(){
                    bool_mask[y as usize][x as usize] = if pixel.0[0] == 0 { false } else { true };
                }

                masks.push(bool_mask);
            }
            _ => panic!("Mask Images should be 8-bit greyscale without alpha")
        }
    }

    let mask_width = masks[0][0].len() as u32;
    let mask_height = masks[0].len() as u32;
    
    // Should we start at 0 or 1?
    let mut start = 0;
    
    for (y, x) in itertools::iproduct!((0..mask_height), (0..mask_width)){
        if masks.iter().all(|mask| !mask[y as usize][x as usize]){
            start = 1;
            break
        }
    }

    // Now let's tile the masks!
    let segments_per_row = num::Integer::div_ceil(&width, &mask_width) as u32;
    let segments_per_mask = (num::Integer::div_ceil(&height, &mask_height) * segments_per_row) as u32;

    let mut container = vec![0 as u8; (BYTES_PER_PIXEL * width * height) as usize];

    for mask in masks{
        let mut pixel_index = 0;

        for y in 0..height{
            for x in 0..width{
                if mask[(y % mask_height) as usize][(x % mask_height) as usize]{
                    let segment_x = x/mask_width;
                    let segment_y = y/mask_height;
                    
                    let segment_index = (start + segment_x + segments_per_row * segment_y);        // TODO: Remove *10

                    fill_pixel_with_segindex(&mut container, pixel_index, segment_index);
                }
                pixel_index += BYTES_PER_PIXEL;
            }
        }    
        start += segments_per_mask;
    }

    return container;
}

fn generate_concentric_circle_grid(width: u32, height: u32, radius: u32) -> Vec<u8>{
    let mut ret_vector = vec![0 as u8; (BYTES_PER_PIXEL as usize) * (height as usize) * (width as usize)];

    let mut pixel_start_index: u32 = 0;
    for y in 0..height{
        for x in 0..width{
            // TODO: Rewrite to fill 4 pixels per sqrt or to use circle drawing algorithm.
            let dist = ((x as f32 - (width as f32/2.0)).powi(2) + (y as f32 - (height as f32/2.0)).powi(2)).sqrt();

            let segment_index = (dist/(radius as f32)).floor() as u32;
            pixel_start_index = fill_pixel_with_segindex(&mut ret_vector, pixel_start_index, segment_index);
        }
    }

    return ret_vector;
}

fn generate_fill_bucket(mask_path: PathBuf) -> Vec<u8>{
    return fill_bucket_grid(image_tools::get_raw_image(mask_path));
}

fn fill_bucket_grid(input_im: RawImage) -> Vec<u8>{
    let mut is_filled = vec![false; (input_im.height * input_im.width) as usize];
    let mut ret_vector = vec![0 as u8; (BYTES_PER_PIXEL as usize) * (input_im.height as usize) * (input_im.width as usize)];

    let mut fill_from = |start_x: u32, start_y: u32, colour_ind: u32, is_filled: &mut Vec<bool>|{
        if start_x >= input_im.width || start_y >= input_im.height{
            return;
        }

        let start_im_index = x_y_to_index(input_im.width, start_x, start_y);
        let start_segment_index = rgb_to_index(input_im.data[start_im_index as usize], input_im.data[start_im_index as usize + 1], input_im.data[start_im_index as usize + 2]);
        let mut to_fill: Vec<(u32, u32)> = vec![];
        to_fill.push((start_x, start_y));

        let mut inner_lop = |to_fill: &mut Vec<(u32, u32)>, fill_pos: (u32, u32)|{
            let image_index = x_y_to_index(input_im.width, fill_pos.0, fill_pos.1);

            if !is_filled[(fill_pos.0 + input_im.height * fill_pos.1) as usize] && rgb_to_index(input_im.data[image_index as usize], input_im.data[image_index as usize + 1], input_im.data[image_index as usize + 2]) == start_segment_index{
                fill_pixel_with_segindex(&mut ret_vector, image_index as u32, colour_ind);
                is_filled[(fill_pos.0 + input_im.height * fill_pos.1) as usize] = true;

                if fill_pos.1 >= 1{
                    to_fill.push((fill_pos.0, fill_pos.1 - 1));
                }
                to_fill.push((fill_pos.0, fill_pos.1 + 1));

                if fill_pos.0 >= 1{
                    to_fill.push((fill_pos.0 - 1, fill_pos.1));
                }
                to_fill.push((fill_pos.0 + 1, fill_pos.1));
            }
        };

        while let Some(pos) = to_fill.pop(){
            inner_lop(&mut to_fill, pos);
        }
    };

    let mut segment_index: u32 = 0;
    for y in 0..input_im.height{
        for x in 0..input_im.width{
            if !is_filled[(x + input_im.height * y) as usize]{
                fill_from(x, y, segment_index, &mut is_filled);
                segment_index += 1;    
            }
        }
    }
    
    return ret_vector;
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
    if container.len() != BYTES_PER_PIXEL as usize*bool_mask.len(){
        return Err(MaskingError::LengthMismatch);
    }

    for i in 0..bool_mask.len(){
        if !bool_mask[i]{
            container[BYTES_PER_PIXEL as usize*i] = false_value.0;
            container[BYTES_PER_PIXEL as usize*i + 1] = false_value.1;
            container[BYTES_PER_PIXEL as usize*i + 2] = false_value.2;
        }
    }

    return Ok(());
}

#[inline]
fn fill_pixel_with_segindex(container: &mut Vec<u8>, pixel_start_index: u32, segment_index: u32) -> u32{
    let segment_color = segment_index_to_rgb(segment_index);
    container[pixel_start_index as usize] = segment_color.0;
    container[pixel_start_index as usize + 1] = segment_color.1;
    container[pixel_start_index as usize + 2] = segment_color.2;
    container[pixel_start_index as usize + 3] = 255;
    return pixel_start_index + BYTES_PER_PIXEL;
}

#[inline]
fn x_y_to_index(width: u32, x: u32, y: u32) -> u32{
    return x + y * width * BYTES_PER_PIXEL;
}