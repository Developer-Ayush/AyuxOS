use crate::color::Color;
use crate::canvas::Canvas;
use fontdue::{Font, FontSettings};

pub struct FontEngine {
    font: Font,
    size: f32,
}

impl FontEngine {
    pub fn load(data: &[u8], size: f32) -> Result<Self, String> {
        let font = Font::from_bytes(data, FontSettings::default()).map_err(|e| e.to_string())?;
        Ok(Self { font, size })
    }

    pub fn draw_text(&self, canvas: &mut Canvas, text: &str, mut x: i32, y: i32, color: Color) {
        for c in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(c, self.size);
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let coverage = bitmap[row * metrics.width + col];
                    if coverage == 0 { continue; }

                    let px = x + metrics.xmin + col as i32;
                    let py = y - metrics.ymin + (row as i32 - metrics.height as i32);

                    let mut c_with_alpha = color;
                    c_with_alpha.a = ((color.a as u32 * coverage as u32) / 255) as u8;
                    canvas.put_pixel(px, py, c_with_alpha);
                }
            }
            x += metrics.advance_width as i32;
        }
    }

    pub fn measure_text(&self, text: &str) -> (u32, u32) {
        let mut width = 0;
        let mut height = 0;
        for c in text.chars() {
            let metrics = self.font.metrics(c, self.size);
            width += metrics.advance_width as u32;
            height = height.max(metrics.height as u32);
        }
        (width, height)
    }
}
