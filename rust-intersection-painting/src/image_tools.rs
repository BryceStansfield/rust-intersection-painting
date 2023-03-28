use std::path::Path;
use std::path::PathBuf;
use std::fs;
use image::{io::Reader as ImageReader, DynamicImage};

// Decomposes an image into (skip, (width, height), pixel_buffer)
pub struct RawImage{
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>
}

pub fn decompose_image(im: DynamicImage) -> RawImage{
    let rgba8_im = im.to_rgba8();
    let dims = rgba8_im.dimensions();
    RawImage{width: dims.0, height: dims.1, data: rgba8_im.into_vec()}
}

// IO
fn raw_image_to_rgba(r: RawImage) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>>{
    return image::ImageBuffer::from_raw(r.width, r.height, r.data).expect("Container not large enough");
}


pub fn save_raw_image(r: RawImage, out_path: PathBuf){
    let image = raw_image_to_rgba(r);
    image.save(out_path).expect("Image didn't save");    
}

pub fn get_raw_image<P: AsRef<Path>>(path: P) -> RawImage{
    return decompose_image(get_image(path));
}

fn get_image<P>(path: P) -> DynamicImage
    where P: AsRef<Path>
{
    return ImageReader::open(path).expect("").decode().expect("");  // TODO: Fix these excepts...
}

fn get_image_from_direntry(dir: &fs::DirEntry) -> DynamicImage{
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


// Image Folder Iterators
pub struct DynamicImageFolderIterator{
    base_iterator: fs::ReadDir
}

impl DynamicImageFolderIterator{
    pub fn new<P: AsRef<Path>>(folder: P) -> Self {
        DynamicImageFolderIterator { base_iterator: std::fs::read_dir(folder).unwrap() }
    }
}

impl Iterator for DynamicImageFolderIterator{
    type Item = (std::ffi::OsString, DynamicImage);

    fn next(&mut self) -> Option<Self::Item> {
        self.base_iterator.next().map(|maybe_dir_entry| {
            let dir_entry = maybe_dir_entry.unwrap();
            (dir_entry.file_name(), get_image_from_direntry(&dir_entry))
        })
    }
}

pub struct RawImageFolderIterator{
    base_iterator: DynamicImageFolderIterator
}

impl RawImageFolderIterator{
    pub fn new<P: AsRef<Path>>(folder: P) -> Self {
        RawImageFolderIterator { base_iterator: DynamicImageFolderIterator::new(folder) }
    }
}

impl Iterator for RawImageFolderIterator{
    type Item = (std::ffi::OsString, RawImage);

    fn next(&mut self) -> Option<Self::Item> {
        self.base_iterator.next().map(|(file_name, dynamic_image)| (file_name, decompose_image(dynamic_image)))
    }
}
