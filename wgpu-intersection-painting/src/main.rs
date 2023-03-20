use std::{sync::Arc, path::Path, ops::Bound, fmt, iter, result};
use itertools::Itertools;
use image::{io::Reader as ImageReader, DynamicImage};

fn main(){
    let background_path = Path::new("E:\\github\\wgpu-intersection-painting\\wgpu-intersection-painting\\src\\test_images\\circle_image_test.png");
    let grid_image = get_image(background_path);
    let raw_grid_image = decompose_image(&grid_image);
    let line_path = Path::new("E:\\github\\wgpu-intersection-painting\\wgpu-intersection-painting\\src\\test_images\\157292-top-minecraft-shaders-background-1920x1080-large-resolution.jpg");
    let line_image = get_image(line_path);
    let raw_line_image = decompose_image(&line_image);
    let bbs = draw_bounding_boxes(&raw_grid_image);

    //print!("{:?}", bbs);

    let averages = cpu_averager(&raw_grid_image, &bbs, &raw_line_image);

    let raw_buffer = cpu_render_to_buffer(&raw_grid_image, &averages);
    let result_image_buffer: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = image::ImageBuffer::from_raw(raw_grid_image.width, raw_grid_image.height, raw_buffer).expect("This is a test run, so I don't care if it panics!");
    let result_path = Path::new("E:\\github\\wgpu-intersection-painting\\wgpu-intersection-painting\\src\\test_images\\circle_result.png");
    let result = result_image_buffer.save(result_path);
}

fn get_image(path: &Path) -> DynamicImage{
    return ImageReader::open(path).expect("").decode().expect("");
}

fn rgb_to_index(r: u8, g: u8, b: u8) -> usize{
    return (r as usize) + (g as usize) * 256 + (b as usize) * 256 * 256;
}

// Decomposes an image into (skip, (width, height), pixel_buffer)
struct RawImage<'a>{
    skip: usize,       // How far between sets of RGB values?
    width: u32,
    height: u32,
    data: &'a Vec<u8>,
    has_alpha: bool
}

fn decompose_image(im: &DynamicImage) -> RawImage{
    return match im{
        DynamicImage::ImageRgb8(rgb_image) => {
                let dims = rgb_image.dimensions();
                RawImage{skip: 3, width: dims.0, height: dims.1, data: rgb_image.as_raw(), has_alpha: false}
            },
        DynamicImage::ImageRgba8(rgba_image) => {
                let dims = rgba_image.dimensions();
                RawImage{skip: 4, width: dims.0, height: dims.1, data: rgba_image.as_raw(), has_alpha: true}
            },
        _ => panic!("Only rgb8 and rgba8 images are supported")
    };
}

fn cpu_averager(grid_image: &RawImage, bounding_boxes: &Vec<BoundingBox>, line_image: &RawImage) -> Vec<u8>{
    let mut sum_vec: Vec<u64> = vec![0; bounding_boxes.len() * 3];
    let mut count_vec: Vec<u32> = vec![0; bounding_boxes.len()];

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

// Using the default height/width type for the image library
#[derive(Copy, Clone, fmt::Debug)]
struct BoundingBox{
    top: u32,
    bot: u32,
    left: u32,
    right: u32
}