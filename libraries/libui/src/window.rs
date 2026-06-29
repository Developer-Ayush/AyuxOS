use libaipc::{AipcClient, AipcMessage, WindowRequest, WindowResponse, WindowEvent};
use libayux::paths;
use libayux::shm::SharedMemory;
use libgraphics::{Canvas, Rect};
use crate::widget::Widget;
use crate::theme::Theme;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io;

pub struct Window {
    _title: String,
    rect: Rect,
    shm: SharedMemory,
    widgets: Arc<Mutex<Vec<Box<dyn Widget + Send>>>>,
    theme: Theme,
    window_id: u32,
    client: Arc<Mutex<AipcClient>>,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> io::Result<Self> {
        let pid = std::process::id();
        let shm_name = format!("win_shm_{}_{}", pid, rand::random::<u32>());
        let shm = SharedMemory::create(&shm_name, (width * height * 4) as usize)?;

        let mut client = AipcClient::connect(paths::WINDOW_SERVER_SOCKET)?;
        let resp = client.request("libui", None, AipcMessage::Window(WindowRequest::CreateWindow {
            title: title.to_string(),
            width,
            height,
            shm_name,
        }))?;

        let window_id = if let AipcMessage::WindowRes(WindowResponse::WindowCreated { window_id }) = resp {
            window_id
        } else {
            return Err(io::Error::other("Failed to create window: Invalid response"));
        };

        let widgets: Arc<Mutex<Vec<Box<dyn Widget + Send>>>> = Arc::new(Mutex::new(Vec::new()));

        let mut client_events = AipcClient::from_stream(client.stream.try_clone()?);
        let widgets_clone = Arc::clone(&widgets);
        thread::spawn(move || {
            while let Ok(Some(envelope)) = client_events.receive_envelope_safe() {
                if let AipcMessage::WindowEvent(event) = envelope.message {
                    if let Ok(mut ws) = widgets_clone.lock() {
                        match event {
                            WindowEvent::Input { event: input_data } => {
                                for widget in ws.iter_mut() {
                                    if widget.handle_event(&input_data) {
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        let client_arc = Arc::new(Mutex::new(client));

        Ok(Self {
            _title: title.to_string(),
            rect: Rect::new(0, 0, width, height),
            shm,
            widgets,
            theme: Theme::default_ayux(),
            window_id,
            client: client_arc,
        })
    }

    pub fn add_widget(&mut self, widget: Box<dyn Widget + Send>) {
        if let Ok(mut ws) = self.widgets.lock() {
            ws.push(widget);
        }
    }

    pub fn get_widgets(&self) -> Arc<Mutex<Vec<Box<dyn Widget + Send>>>> {
        Arc::clone(&self.widgets)
    }

    pub fn render(&mut self) {
        let mut canvas = Canvas::new(self.shm.as_slice_mut(), self.rect.width, self.rect.height, self.rect.width * 4);
        canvas.clear(self.theme.background);
        if let Ok(ws) = self.widgets.lock() {
            for widget in ws.iter() {
                widget.draw(&mut canvas, &self.theme);
            }
        }

        // Notify window server that we are dirty
        if let Ok(mut client) = self.client.lock() {
            let _ = client.send("libui", None, AipcMessage::WindowEvent(WindowEvent::Dirty { window_id: self.window_id }));
        }
    }
}
