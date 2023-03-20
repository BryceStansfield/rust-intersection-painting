use std::{sync::Arc, path::Path, ops::Bound, fmt, iter, result};
use std::path::PathBuf;
use itertools::Itertools;
use image::{io::Reader as ImageReader, DynamicImage};
use clap::Parser;

mod generators;
mod args;
mod stenciler;

fn main(){
    run_cmdline();
}

fn run_cmdline(){
    let arguments = args::Arguments::parse();

    match arguments.command_type{
        args::GeneratorType::GenerateStencil(args::GenerateStencilCommand{width: w, height: h, output: out, generator: g}) => generators::generate_and_save_stencil(w, h, out, g),
        args::GeneratorType::Static(args::StaticCommand{stencil: s, output: out_path, input: in_path}) => static_command(s, in_path, out_path),
        args::GeneratorType::Dynamic(args::DynamicCommand{input: in_path, output: out_path, generator: g}) => dynamic_command(g, in_path, out_path)
    }
}

fn help(){
    println!("Usage:
    intersection_painter dynamic <generator_name_str> <input_path> <output_path>
    or intersection_painter stencil <stencil_path> <input_path> <output_path>")
}

// Command functions
fn static_command(stencil: PathBuf, in_path: PathBuf, out_path: PathBuf){
    let stencil_dyn = get_image(stencil);
    let stencil_image = decompose_image(&stencil_dyn);
    let input_dyn = get_image(in_path);
    let input_image = decompose_image(&input_dyn);

    let out_image = stenciler::cpu_pipeline(&stencil_image, &input_image);
    save_buffer_as_image(out_image, stencil_image.width, stencil_image.height, out_path);
}

// Command functions
fn dynamic_command(generator: args::Generator, in_path: PathBuf, out_path: PathBuf){
    let input_dyn = get_image(in_path);
    let input_image = decompose_image(&input_dyn);

    let width = input_image.width;
    let height = input_image.height;

    let stencil_buffer = generators::generate_stencil(width as usize, height as usize, generator);
    let stencil_image = generators::stencil_to_raw_image(&stencil_buffer, width, height);

    let out_image = stenciler::cpu_pipeline(&stencil_image, &input_image);
    save_buffer_as_image(out_image, stencil_image.width, stencil_image.height, out_path);
}


// Image saving functions
fn container_to_image_buffer(v: Vec<u8>, width: u32, height: u32) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>>{
    return image::ImageBuffer::from_raw(width, height, v).expect("Container not large enough");
}

fn save_buffer_as_image(image: Vec<u8>, width: u32, height: u32, out_path: PathBuf){
    let image = container_to_image_buffer(image, width, height);
    image.save(out_path).expect("Image didn't save");
}

fn get_image<P>(path: P) -> DynamicImage
    where P: AsRef<Path>
{
    return ImageReader::open(path).expect("").decode().expect("");  // TODO: Fix these excepts...
}

// Decomposes an image into (skip, (width, height), pixel_buffer)
pub struct RawImage<'a>{
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

