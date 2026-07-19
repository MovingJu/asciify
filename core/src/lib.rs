mod engine;
mod decode;
use image::DynamicImage;
use wasm_bindgen::prelude::*;

/// 밝은 곳 → 어두운 곳 순서의 ASCII 램프.
/// 밝기(0~255)를 이 문자열의 인덱스로 매핑해서 문자를 고른다.
/// 참고: <https://paulbourke.net/dataformats/asciiart/>
pub const ASCII_RAMP: &str = " .:-=+*#%@";
// pub const ASCII_RAMP: &str = r#" .'`^",:;Il!i><~+_-?][}{1)(|\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"#;

/// 정적 이미지(PNG/JPEG 등) 바이트를 받아 ASCII 아트 문자열로 변환한다.
///
/// `cols`는 출력할 가로 문자 수 (세로는 터미널 글자의 가로:세로 비율(대략 1:2)을
/// 고려해서 자동 계산해야 자연스럽게 보인다 — 이게 이번 과제의 핵심 포인트 중 하나).
///
/// 구현 순서 힌트:
///   1. `image::load_from_memory(bytes)`로 디코딩
///   2. `.resize(cols, rows, FilterType::...)`로 축소 (rows는 원본 비율 + 문자 비율 보정해서 계산)
///   3. `.to_luma8()`로 흑백 변환
///   4. 각 픽셀의 밝기(0~255)를 `ASCII_RAMP` 인덱스로 매핑
///   5. 한 줄씩 문자열로 합쳐서(개행 포함) 리턴
#[wasm_bindgen]
pub fn image_to_ascii(bytes: &[u8], cols: u32) -> String {
    let Ok(img) = decode::load_from_bytes(bytes) else {
        eprintln!("Failed to convert image from bytes.");
        return String::new();
    };
    let img = engine::resize_image(img, cols).into_luma8();
    engine::image_to_string(img)
}

/// 애니메이션 GIF 바이트를 받아, 프레임별 ASCII 아트 + 딜레이(ms)를 JSON으로 반환한다.
///
/// 반환 형식 예시:
///   [{"ascii":"...","delayMs":100}, {"ascii":"...","delayMs":100}, ...]
///
/// 구현 순서 힌트:
///   1. `image::codecs::gif::GifDecoder`로 프레임 목록 디코딩
///   2. 각 프레임을 `image_to_ascii`와 같은 로직으로 변환 (공용 헬퍼 함수로 뽑아서 재사용 권장)
///   3. 각 프레임의 딜레이(Frame::delay())를 ms 단위로 변환
///   4. 결과를 JSON 문자열로 직렬화해서 리턴 (서버 강의에서 쓴 것처럼 수동 포맷도 되고,
///      serde_json을 Cargo.toml에 추가해도 됨)
#[wasm_bindgen]
pub fn gif_to_ascii_frames(bytes: &[u8], cols: u32) -> String {
    let Ok(frames) = decode::gif_decode(bytes) else {
        eprintln!("Failed to load image from bytes.");
        return String::new();
    };
    let result_gif: Vec<engine::AsciiFrame> = frames
        .map(|item| {
            let numer_denom = item.delay().numer_denom_ms();
            let numerator = numer_denom.0 as f64;
            let denominator = numer_denom.1 as f64;
            let img = DynamicImage::ImageRgba8(item.buffer().to_owned());
            let img = engine::resize_image(img, cols).into_luma8();
            (
                engine::image_to_string(img),
                (numerator / denominator).round() as u32,
            )
        })
        .map(|(ascii, delay)| engine::AsciiFrame::new(ascii, delay as usize))
        .collect();
    match serde_json::to_string(&result_gif) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Can't serialize result: {e}");
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GrayImage};
    use std::path::PathBuf;

    const TEST_DIR: &str = "./tests";
    const FILE_NAME: &str = "dodo.jpeg";

    #[test]
    fn gif_pixel_test() {
        use image::codecs::gif::{GifEncoder, Repeat};
        use image::{Delay, Frame, RgbaImage};

        let black_bytes: Vec<u8> = vec![0, 0, 0, 255]; 
        let img = RgbaImage::from_raw(1, 1, black_bytes).unwrap();
        let delay = Delay::from_numer_denom_ms(100, 1);
        let frame = Frame::from_parts(img, 0, 0, delay);

        let mut gif_bytes: Vec<u8> = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut gif_bytes);
            encoder.set_repeat(Repeat::Infinite).unwrap();
            encoder.encode_frames(vec![frame].into_iter()).unwrap();
        }

        let json = gif_to_ascii_frames(&gif_bytes, 1);

        let expected = format!(
            r#"[{{"ascii":"{}","delayMs":100}}]"#,
            ASCII_RAMP.as_bytes()[0] as char
        );
        assert_eq!(json, expected);
    }
    fn load_gif(bytes: &[u8]) -> usize {
        let Ok(frames) = decode::gif_decode(bytes) else {
            panic!("Failed to load image from bytes.");
        };
        frames.count()
    }
    #[test]
    fn load_gif_test() {
        let buf = std::fs::read(format!("{TEST_DIR}/dodo.gif")).expect("read error");
        assert_eq!(load_gif(&buf), 14);
    }

    #[test]
    fn resize_img_test() {
        let img = engine::resize_image(load_image(), 60);
        assert_eq!(img.width(), 60);
        assert_eq!(img.height(), 35);
    }

    #[test]
    fn pixel_255_test() {
        let white_bytes: Vec<u8> = vec![255];
        let img = GrayImage::from_raw(1, 1, white_bytes).unwrap();
        let result = engine::image_to_string(img.into());
        assert_eq!(
            result,
            String::from(ASCII_RAMP.as_bytes()[ASCII_RAMP.len() - 1] as char)
        );
    }

    #[test]
    fn pixel_0_test() {
        let black_bytes: Vec<u8> = vec![0];
        let img = GrayImage::from_raw(1, 1, black_bytes).unwrap();
        let result = engine::image_to_string(img.into());
        assert_eq!(result, String::from(ASCII_RAMP.as_bytes()[0] as char));
    }

    fn load_image() -> DynamicImage {
        decode::load_image(PathBuf::from(format!("{TEST_DIR}/{FILE_NAME}")))
            .expect("Failed to load img")
    }
}
