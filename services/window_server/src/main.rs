use libaipc::{
    AipcClient, AipcEnvelope, AipcHeader, AipcMessage, MessageType,
    WindowRequest, WindowResponse, WindowEvent, InputEventData,
};
use libayux::paths;
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
    dirty: bool,
}

struct WindowServer {
    display: LinuxFramebuffer,
    windows: Vec<Window>,
    next_window_id: u32,
    focused_window_id: Option<u32>,
    mouse_x: i32,
    mouse_y: i32,
    background_dirty: bool,
    cursor_dirty: bool,
    last_mouse_x: i32,
    last_mouse_y: i32,
}

impl WindowServer {
    fn new() -> Result<Self, String> {
        let display = LinuxFramebuffer::new("/dev/fb0")
            .map_err(|e| format!("Could not open /dev/fb0: {:?}", e))?;
        let info = display.get_info().map_err(|e| e.to_string())?;
        Ok(Self {
            display,
            windows: Vec::new(),
            next_window_id: 1,
            focused_window_id: None,
            mouse_x: (info.width / 2) as i32,
            mouse_y: (info.height / 2) as i32,
            background_dirty: true,
            cursor_dirty: true,
            last_mouse_x: -1,
            last_mouse_y: -1,
        })
    }

    fn create_window(&mut self, title: String, width: u32, height: u32, shm_name: String, stream: UnixStream) -> Result<u32, String> {
        let id = self.next_window_id;
        self.next_window_id += 1;

        let shm = SharedMemory::open(&shm_name, (width * height * 4) as usize)
            .map_err(|e| format!("Failed to open SHM {}: {}", shm_name, e))?;

        let window = Window {
            id,
            _title: title,
            rect: Rect::new(100, 100, width, height),
            shm,
            client_stream: stream,
            dirty: true,
        };

        self.windows.push(window);
        self.focused_window_id = Some(id);
        self.background_dirty = true;
        Ok(id)
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
        let info = match self.display.get_info() {
            Ok(i) => i,
            Err(_) => return,
        };

        let mut needs_flip = false;

        // If background is dirty or window list changed significantly, redraw everything
        // For now, let's keep it simple but use optimized blit
        if self.background_dirty || self.windows.iter().any(|w| w.dirty) || self.mouse_x != self.last_mouse_x || self.mouse_y != self.last_mouse_y {
            let mut canvas = Canvas::new(self.display.get_buffer(), info.width, info.height, info.width * 4);

            canvas.clear(Color::rgb(40, 44, 52));

            for window in &mut self.windows {
                let shm_data = window.shm.as_slice_mut();
                canvas.blit(window.rect.x, window.rect.y, shm_data, window.rect.width, window.rect.height, window.rect.width * 4, true);
                canvas.draw_rect(Rect::new(window.rect.x - 1, window.rect.y - 1, window.rect.width + 2, window.rect.height + 2), Color::WHITE);
                window.dirty = false;
            }

            // Draw cursor
            canvas.fill_rect(Rect::new(self.mouse_x, self.mouse_y, 10, 10), Color::RED);

            self.last_mouse_x = self.mouse_x;
            self.last_mouse_y = self.mouse_y;
            self.background_dirty = false;
            needs_flip = true;
        }

        if needs_flip {
            let _ = self.display.flip();
        }
    }
}

fn main() {
    let socket_path = paths::WINDOW_SERVER_SOCKET;
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
        let listener = UnixListener::bind(socket_path).expect("Failed to bind window server socket");
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
        for i in 0..16 {
            if let Ok(dev) = LinuxEvdev::new(&format!("/dev/input/event{}", i)) {
                devices.push(dev);
            }
        }

        let (width, height) = {
            let s = server_input.lock().expect("Failed to lock server_input");
            let info = s.display.get_info().expect("Failed to get display info");
            (info.width as i32, info.height as i32)
        };

        loop {
            let mut any_event = false;
            for dev in &mut devices {
                while let Ok(event) = dev.read_event() {
                    any_event = true;
                    let mut s = server_input.lock().unwrap();
                    match event {
                        InputEvent::Rel { axis, value, .. } => {
                            if axis == 0 { s.mouse_x = (s.mouse_x + value).max(0).min(width - 1); }
                            else if axis == 1 { s.mouse_y = (s.mouse_y + value).max(0).min(height - 1); }
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
            if !any_event {
                thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    });

    let mut last_frame = std::time::Instant::now();
    let frame_target = std::time::Duration::from_micros(16666); // ~60 FPS

    loop {
        {
            let mut s = server.lock().unwrap();
            s.compose();
        }

        let elapsed = last_frame.elapsed();
        if elapsed < frame_target {
            thread::sleep(frame_target - elapsed);
        }
        last_frame = std::time::Instant::now();
    }
}

fn handle_client(server: Arc<Mutex<WindowServer>>, stream: UnixStream) {
    let mut client = libaipc::AipcClient::from_stream(stream);
    let mut window_ids = Vec::new();

    loop {
        match client.receive_envelope_safe() {
            Ok(Some(envelope)) => {
                let mut s = server.lock().unwrap();
                match envelope.message {
                    AipcMessage::Window(req) => {
                        match req {
                            WindowRequest::CreateWindow { title, width, height, shm_name } => {
                                let stream_res = client.stream.try_clone();
                                if let Ok(stream) = stream_res {
                                    match s.create_window(title, width, height, shm_name, stream) {
                                        Ok(id) => {
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
                                        Err(e) => {
                                            eprintln!("Failed to create window: {}", e);
                                        }
                                    }
                                }
                            }
                            WindowRequest::DestroyWindow { window_id } => {
                                s.remove_window(window_id);
                                window_ids.retain(|&id| id != window_id);
                            }
                            _ => {}
                        }
                    }
                    AipcMessage::WindowEvent(WindowEvent::Dirty { window_id }) => {
                        if let Some(win) = s.windows.iter_mut().find(|w| w.id == window_id) {
                            win.dirty = true;
                        }
                    }
                    _ => {}
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
