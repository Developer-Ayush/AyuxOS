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
            for (i, coverage) in bitmap.iter().enumerate() {
                let px = x + metrics.xmin + (i % metrics.width) as i32;
                let py = y - metrics.ymin - (i / metrics.width) as i32 - metrics.height as i32;

                let mut c_with_alpha = color;
                c_with_alpha.a = ((color.a as u32 * *coverage as u32) / 255) as u8;
                canvas.put_pixel(px, py, c_with_alpha);
            }
            x += metrics.advance_width as i32;
        }
    }
}
