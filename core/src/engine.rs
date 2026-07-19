use image::{imageops, DynamicImage, ImageBuffer, Pixel};
use serde::Serialize;

#[derive(Serialize, Default)]
pub(crate) struct AsciiFrame {
    ascii: String,
    #[serde(rename = "delayMs")]
    delay_ms: usize,
}
impl AsciiFrame {
    pub(crate) fn new(ascii: String, delay_ms: usize) -> Self {
        AsciiFrame { ascii, delay_ms }
    }
}
pub(crate) fn image_to_string<T>(img: ImageBuffer<T, Vec<u8>>) -> String
where
    T: Pixel<Subpixel = u8>,
{
    const BRIGHTNESS_STAGE: usize = crate::ASCII_RAMP.len() - 1;
    let mut result_ascii = String::new();
    for row in img.rows() {
        for pixel in row {
            let brightness_idx = (pixel.to_luma().0[0] as usize * BRIGHTNESS_STAGE) / 255;
            result_ascii.push(crate::ASCII_RAMP.as_bytes()[brightness_idx] as char);
        }
        result_ascii.push('\n');
    }
    result_ascii.pop();
    result_ascii
}
pub(crate) fn resize_image(img: DynamicImage, cols: u32) -> DynamicImage {
    let rows = {
        let correlation_factor = 0.5;
        cols as f64 * (img.height() as f64 / img.width() as f64) * correlation_factor
    };
    img.resize_exact(cols, rows.round() as u32, imageops::FilterType::Triangle)
}
