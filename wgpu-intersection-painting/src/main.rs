use std::collections::HashMap;
use std::fs::DirEntry;
use std::path::Path;
use std::path::PathBuf;
use itertools::Itertools;
use image::{io::Reader as ImageReader, DynamicImage};
use clap::Parser;

mod generators;
mod args;
mod stenciler;

fn main(){
    println!("Args = {}", std::env::args().into_iter().join(" "));
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

// Command functions
fn static_command(stencil: PathBuf, in_path: PathBuf, out_path: PathBuf){
    let stencil_image = get_raw_image(stencil);
    let input_image = get_raw_image(in_path);

    let out_image = stenciler::cpu_pipeline(&stencil_image, &input_image);
    save_raw_image(out_image, out_path);
}

// Command functions
fn get_image_from_direntry(dir: &DirEntry) -> DynamicImage{
    let metadata = dir.metadata();

    match metadata{
        Ok(m) => {
            if m.is_file(){
                return get_image(dir.path());
            }
            else{
                panic!("{:?} is not a file", dir.file_name().to_str());
            }
        }
        Err(e) => panic!("Error reading file metadata {}", e)
    }
}

fn get_image_from_pathbuf(p: PathBuf) -> RawImage{
    let input_dyn = get_image(p);
    return decompose_image(input_dyn);
}

fn generate_stencil_from_image(im: &RawImage, generator: &args::Generator) -> RawImage{
    let width = im.width;
    let height = im.height;

    return generators::generate_stencil(width, height, generator);
}

fn dynamic_command(generator: args::Generator, in_path: PathBuf, out_path: PathBuf){
    if !in_path.exists(){
        panic!("Input path doesn't exist")
    }

    if in_path.is_file(){
        if out_path.exists() && !out_path.is_file(){
            panic!("Input is file but output isn't")
        }

        let input_image = get_image_from_pathbuf(in_path);
    
        let stencil_image = generate_stencil_from_image(&input_image, &generator);
    
        let out_image = stenciler::cpu_pipeline(&stencil_image, &input_image);
        save_raw_image(out_image, out_path);
    }
    else if in_path.is_dir(){
        if out_path.exists(){
            if !out_path.is_dir(){
                panic!("Input is folder, but output isn't")
            }
        }
        else{
            std::fs::create_dir_all(out_path.to_owned()).expect("Failed to create output directory");
        }

        let mut stencils: HashMap<(u32, u32), RawImage> = HashMap::new();     // Resolution to stencil.
        
        for file in std::fs::read_dir(in_path).unwrap(){
            if let Err(e) = file{
                panic!("Failed reading input part way through: {}", e)
            }

            let file_entry = file.unwrap();

            let input_dyn = get_image_from_direntry(&file_entry);
            let input_image = decompose_image(input_dyn);
    
            let width = input_image.width;
            let height = input_image.height;
            
            let stencil_image = if let Some(stencil) = stencils.get(&(width, height)){
                stencil
            } else{
                let stencil_image = generators::generate_stencil(width, height, &generator);
            
                stencils.insert((width, height), stencil_image);    
                stencils.get(&(width, height)).unwrap()
            };
            
            let out_image = stenciler::cpu_pipeline(stencil_image, &input_image);
            save_raw_image(out_image, out_path.join(file_entry.file_name()));
        }
    }
    else{
        panic!("Input should be file or folder.")
    }
}


// Image saving functions
fn raw_image_to_image_buffer(r: RawImage) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>>{
    return image::ImageBuffer::from_raw(r.width, r.height, r.data).expect("Container not large enough");
}

fn save_raw_image(r: RawImage, out_path: PathBuf){
    let image = raw_image_to_image_buffer(r);
    image.save(out_path).expect("Image didn't save");
}

fn get_raw_image<P: AsRef<Path>>(path: P) -> RawImage{
    return decompose_image(get_image(path));
}

fn get_image<P>(path: P) -> DynamicImage
    where P: AsRef<Path>
{
    return ImageReader::open(path).expect("").decode().expect("");  // TODO: Fix these excepts...
}

// Decomposes an image into (skip, (width, height), pixel_buffer)
pub struct RawImage{
    skip: usize,       // How far between sets of RGB values?
    width: u32,
    height: u32,
    data: Vec<u8>,
    has_alpha: bool
}

fn decompose_image(im: DynamicImage) -> RawImage{
    return match im{
        DynamicImage::ImageRgb8(rgb_image) => {
                let dims = rgb_image.dimensions();
                RawImage{skip: 3, width: dims.0, height: dims.1, data: rgb_image.into_vec(), has_alpha: false}
            },
        DynamicImage::ImageRgba8(rgba_image) => {
                let dims = rgba_image.dimensions();
                RawImage{skip: 4, width: dims.0, height: dims.1, data: rgba_image.into_vec(), has_alpha: true}
            },
        _ => panic!("Only rgb8 and rgba8 images are supported")
    };
}

