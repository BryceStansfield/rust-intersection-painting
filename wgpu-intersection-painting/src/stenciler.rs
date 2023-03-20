use std::fmt;
use crate::RawImage;
use std::iter;
use itertools::Itertools;

// Utilities:
pub (in crate) fn rgb_to_index(r: u8, g: u8, b: u8) -> usize{
    return (r as usize) + (g as usize) * 256 + (b as usize) * 256 * 256;
}

// CPU Pipeline
pub fn cpu_pipeline(grid_image: &RawImage, line_image: &RawImage) -> Vec<u8>{
    if grid_image.width != line_image.width || grid_image.height != line_image.height{
        unsafe{
            panic!("Grid Image Dims ({}, {}) != Line Image Dims ({}, {})", grid_image.width, grid_image.height, line_image.width, line_image.height);
        }
    }

    let num_segments = count_segments(grid_image);
    let averages =  cpu_averager(grid_image, num_segments, line_image);
    return cpu_render_to_buffer(grid_image, &averages)
}

fn count_segments(image: &RawImage) -> usize{
    let mut max = 0;
    let mut im_index: usize = 0;

    for y in 0..image.height{
        for x in 0..image.width{
            let segment_index = rgb_to_index(image.data[im_index], image.data[im_index+1], image.data[im_index+2]);

            im_index += image.skip; 
            if segment_index > max{
                max = segment_index;
            }
        }
    }

    return max + 1;
}

fn cpu_averager(grid_image: &RawImage, num_segments: usize, line_image: &RawImage) -> Vec<u8>{
    let mut sum_vec: Vec<u64> = vec![0; num_segments * 3];
    let mut count_vec: Vec<u32> = vec![0; num_segments];

    let mut grid_index: usize = 0;
    let mut line_index: usize = 0;
    
    for y in 0..grid_image.height{
        for x in 0..grid_image.width{
            let segment_index = rgb_to_index(grid_image.data[grid_index], grid_image.data[grid_index + 1], grid_image.data[grid_index + 2]);
            let sum_index = segment_index * 3;
            grid_index += grid_image.skip;

            if !line_image.has_alpha || line_image.data[line_index + 3] == 255{       // I don't want to deal with partial transparency just yet
                sum_vec[sum_index] += line_image.data[line_index] as u64;
                sum_vec[sum_index + 1] += line_image.data[line_index + 1] as u64;
                sum_vec[sum_index + 2] += line_image.data[line_index + 2] as u64;
                count_vec[segment_index] += 1;
            }
            line_index += line_image.skip;
        }
    }

    // TODO: Are iterators too slow for my usecase?
    return iter::zip(sum_vec.chunks(3), count_vec).flat_map(|(sum, count)| {
        if count == 0{
            return [0, 0, 0];
        }
        return [(sum[0]/(count as u64)) as u8, (sum[1]/(count as u64)) as u8, (sum[2]/(count as u64)) as u8];
    }).collect();
}

fn cpu_render_to_buffer(grid_image: &RawImage, averages: &Vec<u8>) -> Vec<u8> {
    let mut ret_vector = vec![0 as u8; (grid_image.width as usize) * (grid_image.height as usize) * 3];

    let mut grid_index = 0;
    let mut ret_index = 0;
    
    for y in 0..grid_image.height{
        for x in 0..grid_image.width{
            let segment_index = rgb_to_index(grid_image.data[grid_index], grid_image.data[grid_index+1], grid_image.data[grid_index+2]) * 3;
            grid_index += grid_image.skip;

            ret_vector[ret_index] = averages[segment_index];
            ret_vector[ret_index + 1] = averages[segment_index + 1];
            ret_vector[ret_index + 2] = averages[segment_index + 2];
            ret_index += 3;
        }
    }

    return ret_vector
}

// GPU Pipeline: TODO: Complete.
// Using the default height/width type for the image library
#[derive(Copy, Clone, fmt::Debug)]
struct BoundingBox{
    top: u32,
    bot: u32,
    left: u32,
    right: u32
}

fn draw_bounding_boxes(image: &RawImage) -> Vec<BoundingBox>{
    // Time to build our bounding boxes!
    // Bounds checks should be cheaper than integer modulus, so I'm looping over x,y
    let mut cur_ind: usize = 0;
    let mut bounding_boxes: Vec<Option<BoundingBox>> = vec!();

    for y in 0..image.height{
        for x in 0..image.width{
            let index = rgb_to_index(image.data[cur_ind], image.data[cur_ind + 1], image.data[cur_ind + 2]);
            cur_ind += image.skip;

            while index >= bounding_boxes.len(){
                bounding_boxes.push(None);
            }

            if let Some(bb_opt) = bounding_boxes.get_mut(index){
                match bb_opt{
                    Some(bb) => {
                        // This is ugly, should figure out how to fix it.
                        if bb.top > y{
                            bb.top = y
                        }
                        else if bb.bot < y{
                            bb.bot = y
                        }
                        
                        if bb.left > x{
                            bb.left = x
                        }
                        else if bb.right < x{
                            bb.right = x
                        }
                    },
                    None => *bb_opt = Some(BoundingBox{top: y, bot: y, left: x, right: x})  
                }
            }
        }
    }

    return bounding_boxes.iter().map(|x| {
        match x{
            Some(bb) => *bb,
            None => panic!("Not all indicies have been defined"),
        }
    }).collect();
}