use std::collections::HashMap;
use std::path::PathBuf;
use itertools::Itertools;
use crate::image_tools::{save_raw_image, get_raw_image, RawImage};
use clap::Parser;

mod generators;
mod args;
mod stenciler;
mod image_tools;
mod gpu;

#[tokio::main]
async fn main(){
    println!("Args = {}", std::env::args().into_iter().join(" "));
    run_cmdline();
}

fn run_cmdline(){
    let arguments = args::Arguments::parse();

    match arguments.command_type{
        args::GeneratorType::GenerateStencil(args::GenerateStencilCommand{width: w, height: h, output: out, generator: g}) => generators::generate_and_save_stencil(w, h, out, g),
        args::GeneratorType::Static(args::StaticCommand{stencil: s, alpha_averaging, output: out_path, input: in_path}) => static_command(s, alpha_averaging, in_path, out_path),
        args::GeneratorType::Dynamic(args::DynamicCommand{input: in_path, alpha_averaging, output: out_path, generator: g}) => dynamic_command(g, alpha_averaging, in_path, out_path)
    }
}

// Command functions
fn static_command(stencil: PathBuf, alpha_averaging: bool, in_path: PathBuf, out_path: PathBuf){
    let stencil_image = get_raw_image(stencil);
    let input_image = get_raw_image(in_path);

    let out_image = stenciler::cpu_pipeline(&stencil_image, alpha_averaging, &input_image);
    save_raw_image(out_image, out_path);
}

// Command functions
fn generate_stencil_from_image(im: &RawImage, generator: &args::Generator) -> RawImage{
    let width = im.width;
    let height = im.height;

    return generators::generate_stencil(width, height, generator);
}

fn dynamic_command(generator: args::Generator, alpha_averaging: bool, in_path: PathBuf, out_path: PathBuf){
    if !in_path.exists(){
        panic!("Input path doesn't exist")
    }

    if in_path.is_file(){
        if out_path.exists() && !out_path.is_file(){
            panic!("Input is file but output isn't")
        }

        let input_image = get_raw_image(in_path);
    
        let stencil_image = generate_stencil_from_image(&input_image, &generator);
    
        let out_image = stenciler::cpu_pipeline(&stencil_image, alpha_averaging, &input_image);
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

        let mut stencils: HashMap<(u32, u32), image_tools::RawImage> = HashMap::new();     // Resolution to stencil.

        for (file_name, input_image) in image_tools::RawImageFolderIterator::new(in_path){
            let width = input_image.width;
            let height = input_image.height;
            
            let stencil_image = if let Some(stencil) = stencils.get(&(width, height)){
                stencil
            } else{
                let stencil_image = generators::generate_stencil(width, height, &generator);
            
                stencils.insert((width, height), stencil_image);    
                stencils.get(&(width, height)).unwrap()
            };
            
            let out_image = stenciler::cpu_pipeline(stencil_image, alpha_averaging, &input_image);
            save_raw_image(out_image, out_path.join(file_name));
        }
    }
    else{
        panic!("Input should be file or folder.")
    }
}