mod decode;
use image::{DynamicImage, imageops};
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
pub fn image_to_ascii(bytes: &[u8], cols: u32) -> String {
    let Ok(img) = decode::load_from_bytes(bytes) else {
        eprintln!("Failed to convert image from bytes.");
        return String::new();
    };
    let img = resize_image(img, cols);
    format!("{} * {}", img.width(), img.height())
}
fn resize_image(img: DynamicImage, cols: u32) -> DynamicImage {
    let rows = {
        let correlation_factor = 0.5;
        cols as f64 * (img.height() as f64 / img.width() as f64) * correlation_factor
    };
    img.resize_exact(cols, rows.round() as u32, imageops::FilterType::Triangle)
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
#[allow(unused_variables)] // temporary turn off warnings
pub fn gif_to_ascii_frames(bytes: &[u8], cols: u32) -> String {
    todo!("이슈 #5, #6 — GIF 프레임 분리 + 프레임별 ASCII 변환 구현")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use super::*;

    const TEST_DIR: &str = "./tests";
    const FILE_NAME: &str = "dodo.jpeg";
    
    #[test]
    fn resize_img_test() {
        let img = resize_image(load_image(), 60);
        assert_eq!(img.width(), 60);
        assert_eq!(img.height(), 35);
    }

    fn load_image() -> DynamicImage {
        decode::load_image(PathBuf::from(format!("{TEST_DIR}/{FILE_NAME}"))).expect("Failed to load img")
    }
}