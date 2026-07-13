use image::{DynamicImage, ImageError};
use thiserror::Error;
use image::ImageReader;

/// Error type for decode.
#[derive(Error, Debug)]
pub enum LoadImageError {
    #[error("Failed to load image")]
    Io(#[from] std::io::Error),

    #[error("Failed to decode image")]
    Decode(#[from] ImageError),
}
/// Function for loading image from bytes. 
/// It'll be used in service.
pub fn load_from_bytes(bytes: &[u8]) -> Result<DynamicImage, LoadImageError> {
    let image = image::load_from_memory(bytes)?;
    Ok(image)
}

/// Function for loading image from files system.
pub fn load_image(path: std::path::PathBuf) -> Result<DynamicImage, LoadImageError> {
    let image = ImageReader::open(path)?.decode()?;
    Ok(image)
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    const TEST_DIR: &str = "./tests";

    #[test]
    fn load_image_test() {
        let img =
            load_image(PathBuf::from(format!("{TEST_DIR}/dodo.jpeg"))).expect("Failed to load img");
        img.save(format!("{TEST_DIR}/dodo_test.png"))
            .expect("Failed to save.");
        fs::remove_file(format!("{TEST_DIR}/dodo_test.png")).expect("Failed to remove file");
    }

    #[test]
    fn test_byte_image() {
        let img =
            load_image(PathBuf::from(format!("{TEST_DIR}/dodo.jpeg"))).expect("Failed to load img");

        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("Failed to encode");

        let img = load_from_bytes(&buf).expect("Failed to convert");
        img.save(format!("{TEST_DIR}/dodo_byte.png"))
            .expect("Failed to save.");
        fs::remove_file(format!("{TEST_DIR}/dodo_byte.png")).expect("Failed to remove file");
    }
}
