use libui::{Window, widgets::{Panel, Button}};
use libgraphics::Rect;
use libayux::paths;
use std::thread;
use std::time::Duration;
use std::process::Command;

fn main() {
    libayux::setup_env();

    let mut window = match Window::new("Desktop", 1024, 768) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create desktop window: {}", e);
            std::process::exit(1);
        }
    };

    window.add_widget(Box::new(Panel {
        rect: Rect::new(0, 728, 1024, 40),
    }));

    window.add_widget(Box::new(Button {
        text: "Terminal".to_string(),
        rect: Rect::new(10, 733, 100, 30),
        pressed: false,
        on_click: Some(Box::new(|| {
            Command::new(paths::app_executable("terminal_emulator")).spawn().ok();
        })),
    }));

    loop {
        window.render();
        thread::sleep(Duration::from_millis(100));
    }
}
