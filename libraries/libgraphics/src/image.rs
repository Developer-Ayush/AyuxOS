use crate::color::Color;
use crate::canvas::Canvas;
use image::{GenericImageView};
use std::path::Path;

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<Color>,
}

impl Image {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| e.to_string())?;
        let (width, height) = img.dimensions();
        let mut data = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let p = img.get_pixel(x, y);
                data.push(Color::rgba(p[0], p[1], p[2], p[3]));
            }
        }

        Ok(Self { width, height, data })
    }

    pub fn draw(&self, canvas: &mut Canvas, x: i32, y: i32) {
        for iy in 0..self.height {
            for ix in 0..self.width {
                let color = self.data[(iy * self.width + ix) as usize];
                canvas.put_pixel(x + ix as i32, y + iy as i32, color);
            }
        }
    }
}
