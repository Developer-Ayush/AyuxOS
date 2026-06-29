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
    fn set_focused(&mut self, _focused: bool) {}
    fn is_focused(&self) -> bool { false }
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
    fn set_focused(&mut self, _focused: bool) {}
    fn is_focused(&self) -> bool { false }
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
    fn set_focused(&mut self, _focused: bool) {}
    fn is_focused(&self) -> bool { false }
}

use std::cell::Cell;

pub struct TextBox {
    pub text: std::sync::Arc<std::sync::Mutex<String>>,
    pub rect: Rect,
    pub focused: bool,
    pub shift_pressed: bool,
    pub caret_visible: Cell<bool>,
    pub last_blink: Cell<std::time::Instant>,
}

impl TextBox {
    pub fn new(rect: Rect) -> Self {
        Self {
            text: std::sync::Arc::new(std::sync::Mutex::new(String::new())),
            rect,
            focused: false,
            shift_pressed: false,
            caret_visible: Cell::new(true),
            last_blink: Cell::new(std::time::Instant::now()),
        }
    }
}

impl Widget for TextBox {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        canvas.fill_rect(self.rect, Color::WHITE);
        let border_color = if self.focused { theme.primary } else { theme.border };
        canvas.draw_rect(self.rect, border_color);
        if let Some(ref font) = theme.font {
            if let Ok(t) = self.text.lock() {
                font.draw_text(canvas, &t, self.rect.x + 5, self.rect.y + 20, theme.foreground);

                if self.focused {
                    if self.last_blink.get().elapsed().as_millis() > 500 {
                        self.caret_visible.set(!self.caret_visible.get());
                        self.last_blink.set(std::time::Instant::now());
                    }

                    if self.caret_visible.get() {
                        let text_width = font.measure_text(&t).0;
                        let caret_x = self.rect.x + 5 + text_width as i32;
                        canvas.fill_rect(Rect::new(caret_x, self.rect.y + 5, 2, self.rect.height - 10), theme.foreground);
                    }
                }
            }
        }
    }
    fn handle_event(&mut self, event: &InputEventData) -> bool {
        match event {
            InputEventData::MouseButton { pressed: true, local_x, local_y, .. } => {
                self.focused = self.rect.contains(*local_x, *local_y);
                self.focused
            }
            InputEventData::Key { code, pressed } if self.focused => {
                if *code == 42 || *code == 54 { // Shift
                    self.shift_pressed = *pressed;
                    return true;
                }

                if !*pressed { return true; }

                let mut t = match self.text.lock() {
                    Ok(t) => t,
                    Err(_) => return false,
                };

                match *code {
                    14 => { t.pop(); } // Backspace
                    28 => { self.focused = false; } // Enter
                    1 => { self.focused = false; } // Esc
                    _ => {
                        if let Some(c) = code_to_char(*code, self.shift_pressed) {
                            t.push(c);
                        }
                    }
                }
                true
            }
            _ => false
        }
    }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
    fn set_focused(&mut self, focused: bool) { self.focused = focused; }
    fn is_focused(&self) -> bool { self.focused }
}

pub struct PasswordBox {
    pub text: std::sync::Arc<std::sync::Mutex<String>>,
    pub rect: Rect,
    pub focused: bool,
    pub shift_pressed: bool,
    pub caret_visible: Cell<bool>,
    pub last_blink: Cell<std::time::Instant>,
}

impl PasswordBox {
    pub fn new(rect: Rect) -> Self {
        Self {
            text: std::sync::Arc::new(std::sync::Mutex::new(String::new())),
            rect,
            focused: false,
            shift_pressed: false,
            caret_visible: Cell::new(true),
            last_blink: Cell::new(std::time::Instant::now()),
        }
    }
}

impl Widget for PasswordBox {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme) {
        canvas.fill_rect(self.rect, Color::WHITE);
        let border_color = if self.focused { theme.primary } else { theme.border };
        canvas.draw_rect(self.rect, border_color);
        if let Some(ref font) = theme.font {
            if let Ok(t) = self.text.lock() {
                let masked: String = "*".repeat(t.len());
                font.draw_text(canvas, &masked, self.rect.x + 5, self.rect.y + 20, theme.foreground);

                if self.focused {
                    if self.last_blink.get().elapsed().as_millis() > 500 {
                        self.caret_visible.set(!self.caret_visible.get());
                        self.last_blink.set(std::time::Instant::now());
                    }

                    if self.caret_visible.get() {
                        let masked_width = font.measure_text(&masked).0;
                        let caret_x = self.rect.x + 5 + masked_width as i32;
                        canvas.fill_rect(Rect::new(caret_x, self.rect.y + 5, 2, self.rect.height - 10), theme.foreground);
                    }
                }
            }
        }
    }
    fn handle_event(&mut self, event: &InputEventData) -> bool {
        match event {
            InputEventData::MouseButton { pressed: true, local_x, local_y, .. } => {
                self.focused = self.rect.contains(*local_x, *local_y);
                self.focused
            }
            InputEventData::Key { code, pressed } if self.focused => {
                if *code == 42 || *code == 54 { // Shift
                    self.shift_pressed = *pressed;
                    return true;
                }

                if !*pressed { return true; }

                let mut t = match self.text.lock() {
                    Ok(t) => t,
                    Err(_) => return false,
                };

                match *code {
                    14 => { t.pop(); } // Backspace
                    28 => { self.focused = false; } // Enter
                    1 => { self.focused = false; } // Esc
                    _ => {
                        if let Some(c) = code_to_char(*code, self.shift_pressed) {
                            t.push(c);
                        }
                    }
                }
                true
            }
            _ => false
        }
    }
    fn set_rect(&mut self, rect: Rect) { self.rect = rect; }
    fn get_rect(&self) -> Rect { self.rect }
    fn set_focused(&mut self, focused: bool) { self.focused = focused; }
    fn is_focused(&self) -> bool { self.focused }
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
            let b = match self.buffer.lock() {
                Ok(b) => b,
                Err(_) => return,
            };
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
            InputEventData::MouseButton { pressed: true, local_x, local_y, .. } => {
                if self.rect.contains(*local_x, *local_y) {
                    return true;
                }
                false
            }
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
    fn set_focused(&mut self, _focused: bool) {}
    fn is_focused(&self) -> bool { true }
}

fn code_to_char(code: u16, shift: bool) -> Option<char> {
    if !shift {
        match code {
            2 => Some('1'), 3 => Some('2'), 4 => Some('3'), 5 => Some('4'), 6 => Some('5'),
            7 => Some('6'), 8 => Some('7'), 9 => Some('8'), 10 => Some('9'), 11 => Some('0'),
            12 => Some('-'), 13 => Some('='),
            16 => Some('q'), 17 => Some('w'), 18 => Some('e'), 19 => Some('r'), 20 => Some('t'),
            21 => Some('y'), 22 => Some('u'), 23 => Some('i'), 24 => Some('o'), 25 => Some('p'),
            26 => Some('['), 27 => Some(']'), 43 => Some('\\'),
            30 => Some('a'), 31 => Some('s'), 32 => Some('d'), 33 => Some('f'), 34 => Some('g'),
            35 => Some('h'), 36 => Some('j'), 37 => Some('k'), 38 => Some('l'), 39 => Some(';'),
            40 => Some('\''),
            44 => Some('z'), 45 => Some('x'), 46 => Some('c'), 47 => Some('v'), 48 => Some('b'),
            49 => Some('n'), 50 => Some('m'), 51 => Some(','), 52 => Some('.'), 53 => Some('/'),
            57 => Some(' '),
            _ => None,
        }
    } else {
        match code {
            2 => Some('!'), 3 => Some('@'), 4 => Some('#'), 5 => Some('$'), 6 => Some('%'),
            7 => Some('^'), 8 => Some('&'), 9 => Some('*'), 10 => Some('('), 11 => Some(')'),
            12 => Some('_'), 13 => Some('+'),
            16 => Some('Q'), 17 => Some('W'), 18 => Some('E'), 19 => Some('R'), 20 => Some('T'),
            21 => Some('Y'), 22 => Some('U'), 23 => Some('I'), 24 => Some('O'), 25 => Some('P'),
            26 => Some('{'), 27 => Some('}'), 43 => Some('|'),
            30 => Some('A'), 31 => Some('S'), 32 => Some('D'), 33 => Some('F'), 34 => Some('G'),
            35 => Some('H'), 36 => Some('J'), 37 => Some('K'), 38 => Some('L'), 39 => Some(':'),
            40 => Some('"'),
            44 => Some('Z'), 45 => Some('X'), 46 => Some('C'), 47 => Some('V'), 48 => Some('B'),
            49 => Some('N'), 50 => Some('M'), 51 => Some('<'), 52 => Some('>'), 53 => Some('?'),
            57 => Some(' '),
            _ => None,
        }
    }
}
