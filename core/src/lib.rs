mod engine;
mod decode;
use image::DynamicImage;
use wasm_bindgen::prelude::*;

/// 밝은 곳 → 어두운 곳 순서의 ASCII 램프.
/// 밝기(0~255)를 이 문자열의 인덱스로 매핑해서 문자를 고른다.
/// 참고: <https://paulbourke.net/dataformats/asciiart/>
pub const ASCII_RAMP: &str = " .:-=+*#%@";

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
pub fn image_to_ascii(bytes: &[u8], cols: u32) -> Result<String, JsError> {
    image_to_ascii_impl(bytes, cols).map_err(|e| JsError::new(&e.to_string()))
}

/// `image_to_ascii`의 실제 로직. `JsError`는 실제 JS/wasm 호스트가 있어야 생성 가능해서
/// (네이티브 `cargo test`에서 호출하면 패닉남), 에러 경로까지 네이티브에서 테스트할 수 있도록
/// wasm_bindgen 경계 안쪽 로직을 분리해뒀다.
fn image_to_ascii_impl(bytes: &[u8], cols: u32) -> Result<String, decode::LoadImageError> {
    let img = decode::load_from_bytes(bytes)?;
    decode::check_size(img.width(), img.height())?;
    let img = engine::resize_image(img, cols).into_luma8();
    Ok(engine::image_to_string(img))
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
pub fn gif_to_ascii_frames(bytes: &[u8], cols: u32) -> Result<String, JsError> {
    gif_to_ascii_frames_impl(bytes, cols).map_err(|e| JsError::new(&e))
}

/// `gif_to_ascii_frames`의 실제 로직 (에러 경로까지 네이티브에서 테스트하려는 이유는
/// `image_to_ascii_impl` 위 주석 참고).
fn gif_to_ascii_frames_impl(bytes: &[u8], cols: u32) -> Result<String, String> {
    let (width, height, frames) = decode::gif_decode(bytes).map_err(|e| e.to_string())?;
    decode::check_size(width, height).map_err(|e| e.to_string())?;
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
    serde_json::to_string(&result_gif).map_err(|e| format!("Can't serialize result: {e}"))
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

        let json = gif_to_ascii_frames(&gif_bytes, 1).expect("Failed to convert gif");

        let expected = format!(
            r#"[{{"ascii":"{}","delayMs":100}}]"#,
            ASCII_RAMP.as_bytes()[0] as char
        );
        assert_eq!(json, expected);
    }

    #[test]
    fn oversized_gif_returns_error() {
        use image::codecs::gif::GifEncoder;
        use image::{Frame, RgbaImage};

        // 4500x4500 = 20,250,000픽셀 > MAX_PIXELS(20,000,000)
        let huge_frame = Frame::new(RgbaImage::new(4500, 4500));
        let mut gif_bytes: Vec<u8> = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut gif_bytes);
            encoder
                .encode_frames(vec![huge_frame].into_iter())
                .expect("Failed to encode huge gif");
        }
        assert!(gif_to_ascii_frames_impl(&gif_bytes, 60).is_err());
    }
    fn load_gif(bytes: &[u8]) -> usize {
        let Ok((_, _, frames)) = decode::gif_decode(bytes) else {
            panic!("Failed to load image from bytes.");
        };
        frames.count()
    }

    #[test]
    fn oversized_image_returns_error() {
        // check_size 자체의 경계 판정은 decode.rs 쪽 단위 테스트(실제 버퍼 할당 없음)로 이미 검증했고,
        // 여기서는 image_to_ascii의 실제 로직(image_to_ascii_impl)이 디코딩 결과에 대해 Err를
        // 리턴하는지 엔드투엔드로 확인한다. (공개 wasm_bindgen 함수는 JsError::new가 실제
        // JS/wasm 호스트를 필요로 해서 네이티브 cargo test에서 직접 호출하면 패닉나기 때문에,
        // wasm 경계 안쪽의 impl 함수를 대신 호출한다.)
        // 4500x4500 = 20,250,000픽셀 > MAX_PIXELS(20,000,000) — 상한을 살짝 넘기는 최소 크기로 잡아
        // 불필요하게 큰 버퍼를 할당하지 않는다.
        let huge = image::DynamicImage::new_rgba8(4500, 4500);
        let mut buf = Vec::new();
        huge.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("Failed to encode huge image");
        assert!(image_to_ascii_impl(&buf, 60).is_err());
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
