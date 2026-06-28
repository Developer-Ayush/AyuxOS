use super::common::HalResult;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Clone)]
pub enum InputEvent {
    Key { code: u16, value: i32 },
    Rel { code: u16, value: i32 },
    Abs { code: u16, value: i32 },
}

pub trait InputDevice {
    fn name(&self) -> String;
    fn read_event(&mut self) -> HalResult<InputEvent>;
}

pub struct LinuxEvdev {
    name: String,
    file: File,
}

#[repr(C)]
struct input_event {
    time: [u64; 2],
    type_: u16,
    code: u16,
    value: i32,
}

impl LinuxEvdev {
    pub fn new(path: &str) -> HalResult<Self> {
        let file = File::open(path)?;
        Ok(Self {
            name: format!("Linux Evdev: {}", path),
            file,
        })
    }
}

impl InputDevice for LinuxEvdev {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn read_event(&mut self) -> HalResult<InputEvent> {
        let mut buf = [0u8; std::mem::size_of::<input_event>()];
        self.file.read_exact(&mut buf)?;
        let event: input_event = unsafe { std::mem::transmute(buf) };

        match event.type_ {
            1 => Ok(InputEvent::Key { code: event.code, value: event.value }),
            2 => Ok(InputEvent::Rel { code: event.code, value: event.value }),
            3 => Ok(InputEvent::Abs { code: event.code, value: event.value }),
            _ => Err(super::common::HalError::InvalidOperation(format!("Unknown event type: {}", event.type_))),
        }
    }
}
