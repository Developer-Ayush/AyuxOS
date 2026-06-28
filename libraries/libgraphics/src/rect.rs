use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width as i32 && y >= self.y && y < self.y + self.height as i32
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        let self_max_x = self.x + self.width as i32;
        let self_max_y = self.y + self.height as i32;
        let other_max_x = other.x + other.width as i32;
        let other_max_y = other.y + other.height as i32;

        self.x < other_max_x && self_max_x > other.x && self.y < other_max_y && self_max_y > other.y
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).min(other.y + other.height as i32);

        if x1 < x2 && y1 < y2 {
            Some(Rect::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32))
        } else {
            None
        }
    }
}
