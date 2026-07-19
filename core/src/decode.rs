#[cfg(test)]
use image::ImageReader;
use image::{AnimationDecoder, DynamicImage, Frame, ImageDecoder, ImageError};
use std::io::Cursor;
use thiserror::Error;

/// 픽셀 수 상한 (가로*세로). 실제 사진/짤은 대부분 여유있게 통과하고,
/// 리사이즈/밝기변환 단계에서 브라우저 탭을 멈추게 하거나 메모리를 과도하게
/// 잡아먹을 수 있는 비정상적으로 큰 입력만 여기서 미리 걸러낸다.
pub(crate) const MAX_PIXELS: u64 = 20_000_000; // 약 20메가픽셀 (예: 5000x4000)

/// 디코딩 직후, 리사이즈/밝기변환 전에 호출해서 이미지가 너무 크면 에러로 튕긴다.
pub(crate) fn check_size(width: u32, height: u32) -> Result<(), LoadImageError> {
    let pixels = width as u64 * height as u64;
    if pixels > MAX_PIXELS {
        return Err(LoadImageError::TooLarge { width, height });
    }
    Ok(())
}

pub(crate) fn gif_decode(
    bytes: &[u8],
) -> Result<(u32, u32, impl Iterator<Item = Frame> + use<'_>), LoadImageError> {
    let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))?;
    let (width, height) = decoder.dimensions();
    let frames = decoder.into_frames().filter_map(|item| match item {
        Ok(frame) => Some(frame),
        Err(err) => {
            eprintln!("Failed to convert frame in gif: {err}");
            None
        }
    });
    Ok((width, height, frames))
}

/// Error type for decode.
#[derive(Error, Debug)]
pub(crate) enum LoadImageError {
    #[error("Failed to load image")]
    Io(#[from] std::io::Error),

    #[error("Failed to decode image")]
    Decode(#[from] ImageError),

    #[error("이미지가 너무 큽니다: {width}x{height} (허용 상한 {MAX_PIXELS}픽셀 초과)")]
    TooLarge { width: u32, height: u32 },
}
/// Function for loading image from bytes.
/// It'll be used in service.
pub(crate) fn load_from_bytes(bytes: &[u8]) -> Result<DynamicImage, LoadImageError> {
    let image = image::load_from_memory(bytes)?;
    Ok(image)
}

/// Function for loading image from files system.
#[cfg(test)]
pub(crate) fn load_image(path: std::path::PathBuf) -> Result<DynamicImage, LoadImageError> {
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

    #[test]
    fn oversized_image_rejected() {
        // 실제 픽셀 버퍼를 할당하지 않고, 크기 체크 로직만 합성 치수로 검증한다.
        let result = check_size(20_000, 2_000); // 40,000,000픽셀 > 상한
        assert!(matches!(result, Err(LoadImageError::TooLarge { width: 20_000, height: 2_000 })));
    }

    #[test]
    fn normal_image_size_allowed() {
        assert!(check_size(4000, 3000).is_ok()); // 12메가픽셀, 상한 이내
    }
}
