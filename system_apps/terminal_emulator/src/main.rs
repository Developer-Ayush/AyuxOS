use libui::{Window, widgets::{Terminal}};
use libgraphics::Rect;
use std::thread;
use std::time::Duration;
use std::process::{Command};
use nix::pty::{forkpty};
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd};
use std::sync::{Arc, Mutex};

fn main() {
    libayux::setup_env();

    let mut window = Window::new("Terminal", 800, 600).expect("Failed to create terminal window");

    let buffer = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = Arc::clone(&buffer);

    let pty = unsafe {
        forkpty(None, None).expect("forkpty failed")
    };

    match pty.fork_result {
        nix::unistd::ForkResult::Parent { .. } => {
            let master_fd = pty.master;
            let mut master_file = unsafe { std::fs::File::from_raw_fd(master_fd) };
            let mut master_writer = master_file.try_clone().unwrap();

            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                loop {
                    if let Ok(n) = master_file.read(&mut buf) {
                        if n == 0 { break; }
                        let s = String::from_utf8_lossy(&buf[..n]);
                        let mut b = buffer_clone.lock().unwrap();
                        for c in s.chars() {
                            if c == '\n' {
                                b.push(String::new());
                            } else if c == '\r' {
                                // ignore
                            } else {
                                if b.is_empty() { b.push(String::new()); }
                                b.last_mut().unwrap().push(c);
                            }
                        }
                    } else {
                        break;
                    }
                }
            });

            let terminal_widget = Terminal {
                buffer: Arc::clone(&buffer),
                rect: Rect::new(0, 0, 800, 600),
                on_key: Some(Box::new(move |code| {
                    if let Some(c) = code_to_char(code) {
                        let _ = master_writer.write_all(&[c as u8]);
                    } else if code == 28 { // Enter
                        let _ = master_writer.write_all(b"\n");
                    } else if code == 14 { // Backspace
                        let _ = master_writer.write_all(&[8]);
                    }
                })),
            };

            window.add_widget(Box::new(terminal_widget));

            loop {
                window.render();
                thread::sleep(Duration::from_millis(32));
            }
        }
        nix::unistd::ForkResult::Child => {
            let _ = Command::new("/bin/ayux_shell").status();
            std::process::exit(0);
        }
    }
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
