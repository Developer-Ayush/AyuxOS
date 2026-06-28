use libgraphics::{Canvas, Color, Rect};
use crate::widget::Widget;
use crate::theme::Theme;
use libaipc::InputEventData;

pub struct Label {
    pub text: String,
    pub rect: Rect,
}

impl Widget for Label {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        canvas.fill_rect(self.rect, theme.background);
        if let Some(ref font) = theme.font {
            font.draw_text(canvas, &self.text, self.rect.x, self.rect.y + 16, theme.foreground);
        }
    }
    fn handle_event(&mut self, _event: &InputEventData) -> bool { false }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
}

pub struct Button {
    pub text: String,
    pub rect: Rect,
    pub pressed: bool,
    pub on_click: Option<Box<dyn FnMut() + Send>>,
}

impl Widget for Button {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        let color = if self.pressed { theme.primary } else { theme.border };
        canvas.fill_rect(self.rect, color);
        canvas.draw_rect(self.rect, theme.foreground);
        if let Some(ref font) = theme.font {
            font.draw_text(canvas, &self.text, self.rect.x + 5, self.rect.y + 25, theme.foreground);
        }
    }
    fn handle_event(&mut self, event: &InputEventData) -> bool {
        match event {
            InputEventData::MouseButton { pressed, local_x, local_y, .. } => {
                if self.rect.contains(*local_x, *local_y) {
                    self.pressed = *pressed;
                    if !*pressed {
                        if let Some(ref mut cb) = self.on_click {
                            cb();
                        }
                    }
                    return true;
                }
                self.pressed = false;
                false
            }
            _ => false
        }
    }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
}

pub struct Panel {
    pub rect: Rect,
}

impl Widget for Panel {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        canvas.fill_rect(self.rect, theme.background);
        canvas.draw_rect(self.rect, theme.border);
    }
    fn handle_event(&mut self, _event: &InputEventData) -> bool { false }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
}

pub struct TextBox {
    pub text: std::sync::Arc<std::sync::Mutex<String>>,
    pub rect: Rect,
    pub focused: bool,
}

impl TextBox {
    pub fn new(rect: Rect) -> Self {
        Self {
            text: std::sync::Arc::new(std::sync::Mutex::new(String::new())),
            rect,
            focused: false,
        }
    }
}

impl Widget for TextBox {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        canvas.fill_rect(self.rect, Color::WHITE);
        let border_color = if self.focused { theme.primary } else { theme.border };
        canvas.draw_rect(self.rect, border_color);
        if let Some(ref font) = theme.font {
            let t = self.text.lock().unwrap();
            font.draw_text(canvas, &t, self.rect.x + 5, self.rect.y + 20, theme.foreground);
        }
    }
    fn handle_event(&mut self, event: &InputEventData) -> bool {
        match event {
            InputEventData::MouseButton { pressed: true, local_x, local_y, .. } => {
                self.focused = self.rect.contains(*local_x, *local_y);
                self.focused
            }
            InputEventData::Key { code, pressed: true } if self.focused => {
                let mut t = self.text.lock().unwrap();
                if *code == 14 { t.pop(); }
                else if *code == 28 { self.focused = false; }
                else if let Some(c) = code_to_char(*code) {
                    t.push(c);
                }
                true
            }
            _ => false
        }
    }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
}

pub struct PasswordBox {
    pub text: std::sync::Arc<std::sync::Mutex<String>>,
    pub rect: Rect,
    pub focused: bool,
}

impl PasswordBox {
    pub fn new(rect: Rect) -> Self {
        Self {
            text: std::sync::Arc::new(std::sync::Mutex::new(String::new())),
            rect,
            focused: false,
        }
    }
}

impl Widget for PasswordBox {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        canvas.fill_rect(self.rect, Color::WHITE);
        let border_color = if self.focused { theme.primary } else { theme.border };
        canvas.draw_rect(self.rect, border_color);
        if let Some(ref font) = theme.font {
            let t = self.text.lock().unwrap();
            let masked: String = "*".repeat(t.len());
            font.draw_text(canvas, &masked, self.rect.x + 5, self.rect.y + 20, theme.foreground);
        }
    }
    fn handle_event(&mut self, event: &InputEventData) -> bool {
        match event {
            InputEventData::MouseButton { pressed: true, local_x, local_y, .. } => {
                self.focused = self.rect.contains(*local_x, *local_y);
                self.focused
            }
            InputEventData::Key { code, pressed: true } if self.focused => {
                let mut t = self.text.lock().unwrap();
                if *code == 14 { t.pop(); }
                else if *code == 28 { self.focused = false; }
                else if let Some(c) = code_to_char(*code) {
                    t.push(c);
                }
                true
            }
            _ => false
        }
    }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
}

pub struct Terminal {
    pub buffer: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    pub rect: Rect,
    pub on_key: Option<Box<dyn FnMut(u16) + Send>>,
}

impl Widget for Terminal {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        canvas.fill_rect(self.rect, Color::BLACK);
        if let Some(ref font) = theme.font {
            let b = self.buffer.lock().unwrap();
            let mut y = self.rect.y + 20;
            // Draw last N lines that fit
            let max_lines = (self.rect.height / 20) as usize;
            let start = if b.len() > max_lines { b.len() - max_lines } else { 0 };
            for line in &b[start..] {
                font.draw_text(canvas, line, self.rect.x + 5, y, Color::WHITE);
                y += 20;
            }
        }
    }
    fn handle_event(&mut self, event: &InputEventData) -> bool {
        match event {
            InputEventData::Key { code, pressed: true } => {
                if let Some(ref mut cb) = self.on_key {
                    cb(*code);
                }
                true
            }
            _ => false
        }
    }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
}

fn code_to_char(code: u16) -> Option<char> {
    match code {
        16..=25 => Some("qwertyuiop".chars().nth((code - 16) as usize).unwrap()),
        30..=38 => Some("asdfghjkl".chars().nth((code - 30) as usize).unwrap()),
        44..=50 => Some("zxcvbnm".chars().nth((code - 44) as usize).unwrap()),
        57 => Some(' '),
        2..=11 => Some("1234567890".chars().nth((code - 2) as usize).unwrap()),
        _ => None,
    }
}
