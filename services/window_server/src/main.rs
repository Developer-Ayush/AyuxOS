use libaipc::{
    AipcClient, AipcEnvelope, AipcHeader, AipcMessage, MessageType,
    WindowRequest, WindowResponse, WindowEvent, InputEventData,
};
use libayux::shm::SharedMemory;
use libayux_hal::display::{Display, LinuxFramebuffer};
use libayux_hal::input::{InputDevice, LinuxEvdev, InputEvent};
use libgraphics::{Canvas, Color, Rect};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;

struct Window {
    id: u32,
    _title: String,
    rect: Rect,
    shm: SharedMemory,
    client_stream: UnixStream,
}

struct WindowServer {
    display: LinuxFramebuffer,
    windows: Vec<Window>,
    next_window_id: u32,
    focused_window_id: Option<u32>,
    mouse_x: i32,
    mouse_y: i32,
}

impl WindowServer {
    fn new() -> Result<Self, String> {
        let display = LinuxFramebuffer::new("/dev/fb0")
            .map_err(|e| format!("Could not open /dev/fb0: {:?}", e))?;
        Ok(Self {
            display,
            windows: Vec::new(),
            next_window_id: 1,
            focused_window_id: None,
            mouse_x: 512,
            mouse_y: 384,
        })
    }

    fn create_window(&mut self, title: String, width: u32, height: u32, shm_name: String, stream: UnixStream) -> u32 {
        let id = self.next_window_id;
        self.next_window_id += 1;

        let shm = SharedMemory::open(&shm_name, (width * height * 4) as usize).expect("Failed to open SHM");

        let window = Window {
            id,
            _title: title,
            rect: Rect::new(100, 100, width, height),
            shm,
            client_stream: stream,
        };

        self.windows.push(window);
        self.focused_window_id = Some(id);
        id
    }

    fn remove_window(&mut self, id: u32) {
        self.windows.retain(|w| w.id != id);
        if self.focused_window_id == Some(id) {
            self.focused_window_id = self.windows.last().map(|w| w.id);
        }
    }

    fn dispatch_input(&mut self, event_type: &str, button: Option<u16>, pressed: Option<bool>, code: Option<u16>) {
        let mut target_window_id = None;
        let mut local_x = 0;
        let mut local_y = 0;

        if event_type == "mouse" {
            for window in self.windows.iter().rev() {
                if window.rect.contains(self.mouse_x, self.mouse_y) {
                    target_window_id = Some(window.id);
                    local_x = self.mouse_x - window.rect.x;
                    local_y = self.mouse_y - window.rect.y;
                    break;
                }
            }
        } else {
            target_window_id = self.focused_window_id;
        }

        if let Some(wid) = target_window_id {
            if let Some(win) = self.windows.iter_mut().find(|w| w.id == wid) {
                let event_data = if event_type == "mouse_move" {
                    InputEventData::MouseMove { x: self.mouse_x, y: self.mouse_y, local_x, local_y }
                } else if event_type == "mouse_button" {
                    InputEventData::MouseButton {
                        button: button.unwrap(),
                        pressed: pressed.unwrap(),
                        x: self.mouse_x,
                        y: self.mouse_y,
                        local_x,
                        local_y
                    }
                } else {
                    InputEventData::Key { code: code.unwrap(), pressed: pressed.unwrap() }
                };

                if let Ok(stream) = win.client_stream.try_clone() {
                    let mut client = AipcClient::from_stream(stream);
                    let env = AipcEnvelope {
                        header: AipcHeader {
                            version: libaipc::AIPC_VERSION,
                            message_type: MessageType::Event,
                            sender: "window_server".into(),
                            session_id: None,
                            correlation_id: 0,
                        },
                        message: AipcMessage::WindowEvent(WindowEvent::Input { event: event_data }),
                    };
                    let _ = client.send_envelope(&env);
                }
            }
        }
    }

    fn compose(&mut self) {
        let info = self.display.get_info().unwrap();
        let mut canvas = Canvas::new(self.display.get_buffer(), info.width, info.height, info.pitch);

        canvas.clear(Color::rgb(40, 44, 52));

        for window in &mut self.windows {
            let win_w = window.rect.width;
            let win_h = window.rect.height;
            let shm_data = window.shm.as_slice_mut();

            for win_y in 0..win_h {
                for win_x in 0..win_w {
                    let offset = (win_y * win_w * 4 + win_x * 4) as usize;
                    let b = shm_data[offset];
                    let g = shm_data[offset + 1];
                    let r = shm_data[offset + 2];
                    let a = shm_data[offset + 3];
                    canvas.put_pixel(window.rect.x + win_x as i32, window.rect.y + win_y as i32, Color::rgba(r, g, b, a));
                }
            }
            canvas.draw_rect(Rect::new(window.rect.x - 1, window.rect.y - 1, window.rect.width + 2, window.rect.height + 2), Color::WHITE);
        }

        canvas.fill_rect(Rect::new(self.mouse_x, self.mouse_y, 10, 10), Color::RED);
        let _ = self.display.flip();
    }
}

fn main() {
    let socket_path = "/run/window_server.sock";
    if fs::metadata(socket_path).is_ok() {
        let _ = fs::remove_file(socket_path);
    }

    let server_instance = match WindowServer::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Window Server failed to initialize: {}", e);
            std::process::exit(1);
        }
    };

    let server = Arc::new(Mutex::new(server_instance));

    let server_ipc = Arc::clone(&server);
    thread::spawn(move || {
        let listener = UnixListener::bind(socket_path).unwrap();
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let server_inner = Arc::clone(&server_ipc);
                thread::spawn(move || {
                    handle_client(server_inner, stream);
                });
            }
        }
    });

    let server_input = Arc::clone(&server);
    thread::spawn(move || {
        let mut devices = Vec::new();
        for i in 0..10 {
            if let Ok(dev) = LinuxEvdev::new(&format!("/dev/input/event{}", i)) {
                devices.push(dev);
            }
        }

        loop {
            for dev in &mut devices {
                while let Ok(event) = dev.read_event() {
                    let mut s = server_input.lock().unwrap();
                    match event {
                        InputEvent::Rel { axis, value, .. } => {
                            if axis == 0 { s.mouse_x = (s.mouse_x + value).max(0).min(1024); }
                            else if axis == 1 { s.mouse_y = (s.mouse_y + value).max(0).min(768); }
                            s.dispatch_input("mouse_move", None, None, None);
                        }
                        InputEvent::Key { code, value } => {
                            if code >= 0x110 && code <= 0x112 {
                                s.dispatch_input("mouse_button", Some(code), Some(value != 0), None);
                            } else {
                                s.dispatch_input("key", None, Some(value != 0), Some(code));
                            }
                        }
                        _ => {}
                    }
                }
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    loop {
        {
            let mut s = server.lock().unwrap();
            s.compose();
        }
        thread::sleep(std::time::Duration::from_millis(16));
    }
}

fn handle_client(server: Arc<Mutex<WindowServer>>, stream: UnixStream) {
    let mut client = libaipc::AipcClient::from_stream(stream);
    let mut window_ids = Vec::new();

    loop {
        match client.receive_envelope_safe() {
            Ok(Some(envelope)) => {
                if let AipcMessage::Window(req) = envelope.message {
                    let mut s = server.lock().unwrap();
                    match req {
                        WindowRequest::CreateWindow { title, width, height, shm_name } => {
                            let id = s.create_window(title, width, height, shm_name, client.stream.try_clone().unwrap());
                            window_ids.push(id);
                            let res = AipcMessage::WindowRes(WindowResponse::WindowCreated { window_id: id });
                            let _ = client.send_envelope(&AipcEnvelope {
                                header: AipcHeader {
                                    version: libaipc::AIPC_VERSION,
                                    message_type: MessageType::Response,
                                    sender: "window_server".into(),
                                    session_id: None,
                                    correlation_id: envelope.header.correlation_id,
                                },
                                message: res,
                            });
                        }
                        WindowRequest::DestroyWindow { window_id } => {
                            s.remove_window(window_id);
                            window_ids.retain(|&id| id != window_id);
                        }
                        _ => {}
                    }
                }
            }
            _ => break,
        }
    }

    let mut s = server.lock().unwrap();
    for id in window_ids {
        s.remove_window(id);
    }
}
