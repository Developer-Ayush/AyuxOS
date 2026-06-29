use libgraphics::{Color, font::FontEngine};
use std::sync::Arc;

pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub accent: Color,
    pub border: Color,
    pub padding: u32,
    pub margin: u32,
    pub font: Option<Arc<FontEngine>>,
}

impl Theme {
    pub fn default_ayux() -> Self {
        use libayux::paths;
        let font_path = format!("{}/default.ttf", paths::AYUX_FONTS);
        let font_data = std::fs::read(font_path).ok();
        let font = font_data.and_then(|data| FontEngine::load(&data, 16.0).ok()).map(Arc::new);

        Self {
            background: Color::rgb(240, 240, 240),
            foreground: Color::rgb(33, 33, 33),
            primary: Color::rgb(33, 150, 243),
            accent: Color::rgb(255, 64, 129),
            border: Color::rgb(200, 200, 200),
            padding: 8,
            margin: 8,
            font,
        }
    }
}
