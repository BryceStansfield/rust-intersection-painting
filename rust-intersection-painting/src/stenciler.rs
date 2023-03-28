use std::fmt;
use crate::image_tools::RawImage;
use crate::generators::BYTES_PER_PIXEL;
use std::iter;

// Utilities:
pub (in crate) fn rgb_to_index(r: u8, g: u8, b: u8) -> usize{
    return (r as usize) + (g as usize) * 256 + (b as usize) * 256 * 256;
}

// CPU Pipeline
pub fn cpu_pipeline(grid_image: &RawImage, alpha_averaging: bool, line_image: &RawImage) -> RawImage{
    if grid_image.width != line_image.width || grid_image.height != line_image.height{
        panic!("Grid Image Dims ({}, {}) != Line Image Dims ({}, {})", grid_image.width, grid_image.height, line_image.width, line_image.height);
    }

    let num_segments = count_segments(grid_image);
    let averages =  cpu_averager(grid_image, num_segments, alpha_averaging, line_image);
    let buffer = cpu_render_to_buffer(grid_image, &averages);

    return RawImage { width: grid_image.width, height: grid_image.height, data: buffer }
}

fn count_segments(image: &RawImage) -> usize{
    let mut max = 0;
    let mut im_index: usize = 0;

    for _y in 0..image.height{
        for _x in 0..image.width{
            let segment_index = rgb_to_index(image.data[im_index], image.data[im_index+1], image.data[im_index+2]);

            im_index += BYTES_PER_PIXEL as usize; 
            if segment_index > max{
                max = segment_index;
            }
        }
    }

    return max + 1;
}

fn cpu_averager(grid_image: &RawImage, num_segments: usize, alpha_averaging: bool, line_image: &RawImage) -> Vec<u8>{
    let mut sum_vec: Vec<u64> = vec![0; num_segments * 4];
    let mut count_vec: Vec<u32> = vec![0; num_segments];

    let mut grid_index: usize = 0;
    let mut line_index: usize = 0;
    
    for _y in 0..grid_image.height{
        for _x in 0..grid_image.width{
            let segment_index = rgb_to_index(grid_image.data[grid_index], grid_image.data[grid_index + 1], grid_image.data[grid_index + 2]);
            let sum_index = segment_index * 4;
            grid_index += BYTES_PER_PIXEL as usize;

            if alpha_averaging{
                sum_vec[sum_index] += line_image.data[line_index] as u64;
                sum_vec[sum_index + 1] += line_image.data[line_index + 1] as u64;
                sum_vec[sum_index + 2] += line_image.data[line_index + 2] as u64;
                sum_vec[sum_index + 3] += line_image.data[line_index + 3] as u64;
                count_vec[segment_index] += 1
            }
            else{
                if line_image.data[line_index + 3] != 0{
                    sum_vec[sum_index] += line_image.data[line_index] as u64;
                    sum_vec[sum_index + 1] += line_image.data[line_index + 1] as u64;
                    sum_vec[sum_index + 2] += line_image.data[line_index + 2] as u64;
                    sum_vec[sum_index + 3] += 255 as u64;
                    count_vec[segment_index] += 1
                }
            }
            
            line_index += BYTES_PER_PIXEL as usize;
        }
    }

    // TODO: Are iterators too slow for my usecase?
    return iter::zip(sum_vec.chunks(4), count_vec).flat_map(|(sum, count)| {
        if count == 0{
            return [0, 0, 0, 255];
        }
        return [(sum[0]/(count as u64)) as u8, (sum[1]/(count as u64)) as u8, (sum[2]/(count as u64)) as u8, (sum[3]/(count as u64)) as u8];
    }).collect();
}

fn cpu_render_to_buffer(grid_image: &RawImage, averages: &Vec<u8>) -> Vec<u8> {
    let mut ret_vector = vec![0 as u8; (grid_image.width as usize) * (grid_image.height as usize) * BYTES_PER_PIXEL as usize];

    let mut grid_index = 0;
    let mut ret_index = 0;
    
    for _y in 0..grid_image.height{
        for _x in 0..grid_image.width{
            let segment_index = rgb_to_index(grid_image.data[grid_index], grid_image.data[grid_index+1], grid_image.data[grid_index+2]) * BYTES_PER_PIXEL as usize;
            grid_index += BYTES_PER_PIXEL as usize;

            ret_vector[ret_index] = averages[segment_index];
            ret_vector[ret_index + 1] = averages[segment_index + 1];
            ret_vector[ret_index + 2] = averages[segment_index + 2];
            ret_vector[ret_index + 3] = averages[segment_index + 3];
            ret_index += BYTES_PER_PIXEL as usize;
        }
    }

    return ret_vector
}

// GPU Pipeline: TODO: Complete.
/*pub fn gpu_pipeline(grid_image: &RawImage, line_image: &RawImage) -> RawImage{
    let bbs = draw_bounding_boxes(grid_image);

}*/


// Using the default height/width type for the image library
#[derive(Copy, Clone, fmt::Debug)]
struct BoundingBox{
    top: u32,
    bot: u32,
    left: u32,
    right: u32
}

#[allow(unused)]
fn draw_bounding_boxes(image: &RawImage) -> Vec<BoundingBox>{
    // Time to build our bounding boxes!
    // Bounds checks should be cheaper than integer modulus, so I'm looping over x,y
    let mut cur_ind: usize = 0;
    let mut bounding_boxes: Vec<Option<BoundingBox>> = vec!();
    let mut max_bb_size: (u32, u32) = (0,0);    // We want our bounding_boxes to be of equal size.

    for y in 0..image.height{
        for x in 0..image.width{
            let index = rgb_to_index(image.data[cur_ind], image.data[cur_ind + 1], image.data[cur_ind + 2]);
            cur_ind += BYTES_PER_PIXEL as usize;

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

                        if bb.bot - bb.top > max_bb_size.1{
                            max_bb_size = (max_bb_size.0, bb.bot - bb.top);
                        }
                        
                        if bb.left > x{
                            bb.left = x
                        }
                        else if bb.right < x{
                            bb.right = x
                        }

                        if bb.right - bb.left > max_bb_size.0{
                            max_bb_size = (bb.right - bb.left, max_bb_size.1);
                        }

                    },
                    None => *bb_opt = Some(BoundingBox{top: y, bot: y, left: x, right: x})  
                }
            }
        }
    }

    let mut bounding_boxes: Vec<BoundingBox> = bounding_boxes.iter().map(|x| {
        match x{
            Some(bb) => *bb,
            None => panic!("Not all indicies have been defined"),
        }
    }).collect();

    // Now time to readjust our bounding boxes to all be the same size.
    // TODO: Is this strictly neccessary? Seems like it shouldn't be.
    for bb in bounding_boxes.iter_mut(){
        // Adjusting the y axis.
        let y_size_dist = (bb.bot - bb.top) - max_bb_size.1;
        if y_size_dist != 0{
            if bb.top >= y_size_dist{
                bb.top -= y_size_dist;
            }
            else{
                bb.top = 0;
                bb.bot += y_size_dist - bb.top;
            }
        }

        // Adjusting the x axis.
        let x_size_dist = (bb.right - bb.left) - max_bb_size.0;
        if x_size_dist != 0{
            if bb.left >= x_size_dist{
                bb.left -= x_size_dist;
            }
            else{
                bb.left = 0;
                bb.right += x_size_dist - bb.left;
            }
        }

    }

    return bounding_boxes;
}