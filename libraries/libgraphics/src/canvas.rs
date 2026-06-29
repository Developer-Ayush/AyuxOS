use crate::color::Color;
use crate::rect::Rect;

pub struct Canvas<'a> {
    pub buffer: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub clip_rect: Rect,
}

impl<'a> Canvas<'a> {
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32, pitch: u32) -> Self {
        Self {
            buffer,
            width,
            height,
            pitch,
            clip_rect: Rect::new(0, 0, width, height),
        }
    }

    pub fn set_clip(&mut self, rect: Rect) {
        self.clip_rect = rect.intersection(&Rect::new(0, 0, self.width, self.height))
            .unwrap_or(Rect::new(0, 0, 0, 0));
    }

    pub fn clear(&mut self, color: Color) {
        self.fill_rect(self.clip_rect, color);
    }

    #[inline]
    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if !self.clip_rect.contains(x, y) {
            return;
        }

        let offset = (y as u32 * self.pitch + x as u32 * 4) as usize;
        if color.a == 255 {
            self.buffer[offset] = color.b;
            self.buffer[offset + 1] = color.g;
            self.buffer[offset + 2] = color.r;
            self.buffer[offset + 3] = 255;
        } else if color.a > 0 {
            // Alpha blending
            let alpha = color.a as u32;
            let inv_alpha = 255 - alpha;

            // Use faster blending
            let b = ((color.b as u32 * alpha + self.buffer[offset] as u32 * inv_alpha + 127) / 255) as u8;
            let g = ((color.g as u32 * alpha + self.buffer[offset + 1] as u32 * inv_alpha + 127) / 255) as u8;
            let r = ((color.r as u32 * alpha + self.buffer[offset + 2] as u32 * inv_alpha + 127) / 255) as u8;

            self.buffer[offset] = b;
            self.buffer[offset + 1] = g;
            self.buffer[offset + 2] = r;
            self.buffer[offset + 3] = 255; // Destination is always opaque for now
        }
    }

    pub fn blit(&mut self, x: i32, y: i32, src_buffer: &[u8], src_w: u32, src_h: u32, src_pitch: u32, has_alpha: bool) {
        let dest_rect = Rect::new(x, y, src_w, src_h);
        let intersection = match dest_rect.intersection(&self.clip_rect) {
            Some(r) => r,
            None => return,
        };

        let start_x = intersection.x - x;
        let start_y = intersection.y - y;

        for row in 0..intersection.height as i32 {
            let src_row = start_y + row;
            let dest_row = intersection.y + row;

            let src_offset = (src_row as u32 * src_pitch + start_x as u32 * 4) as usize;
            let dest_offset = (dest_row as u32 * self.pitch + intersection.x as u32 * 4) as usize;

            if !has_alpha {
                // Fast copy
                let copy_len = (intersection.width * 4) as usize;
                self.buffer[dest_offset..dest_offset + copy_len]
                    .copy_from_slice(&src_buffer[src_offset..src_offset + copy_len]);
            } else {
                // Alpha blit
                for col in 0..intersection.width as i32 {
                    let s_off = src_offset + (col as usize * 4);
                    let b = src_buffer[s_off];
                    let g = src_buffer[s_off + 1];
                    let r = src_buffer[s_off + 2];
                    let a = src_buffer[s_off + 3];
                    self.put_pixel(intersection.x + col, dest_row, Color::rgba(r, g, b, a));
                }
            }
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let intersection = match rect.intersection(&self.clip_rect) {
            Some(r) => r,
            None => return,
        };

        for y in intersection.y..(intersection.y + intersection.height as i32) {
            for x in intersection.x..(intersection.x + intersection.width as i32) {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x1;
        let mut y = y1;

        loop {
            self.put_pixel(x, y, color);
            if x == x2 && y == y2 { break; }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        let x2 = rect.x + rect.width as i32 - 1;
        let y2 = rect.y + rect.height as i32 - 1;
        self.draw_line(rect.x, rect.y, x2, rect.y, color);
        self.draw_line(x2, rect.y, x2, y2, color);
        self.draw_line(x2, y2, rect.x, y2, color);
        self.draw_line(rect.x, y2, rect.x, rect.y, color);
    }

    pub fn draw_circle(&mut self, xc: i32, yc: i32, r: i32, color: Color) {
        let mut x = 0;
        let mut y = r;
        let mut d = 3 - 2 * r;
        self.draw_circle_points(xc, yc, x, y, color);
        while y >= x {
            x += 1;
            if d > 0 {
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
            self.draw_circle_points(xc, yc, x, y, color);
        }
    }

    fn draw_circle_points(&mut self, xc: i32, yc: i32, x: i32, y: i32, color: Color) {
        self.put_pixel(xc + x, yc + y, color);
        self.put_pixel(xc - x, yc + y, color);
        self.put_pixel(xc + x, yc - y, color);
        self.put_pixel(xc - x, yc - y, color);
        self.put_pixel(xc + y, yc + x, color);
        self.put_pixel(xc - y, yc + x, color);
        self.put_pixel(xc + y, yc - x, color);
        self.put_pixel(xc - y, yc - x, color);
    }
}
