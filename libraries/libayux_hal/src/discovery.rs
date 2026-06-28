use super::common::HalResult;
use std::fs;

pub struct Device {
    pub name: String,
    pub class: String,
    pub path: String,
}

pub trait Discovery {
    fn scan(&self) -> HalResult<Vec<Device>>;
}

pub struct LinuxDiscovery;

impl Discovery for LinuxDiscovery {
    fn scan(&self) -> HalResult<Vec<Device>> {
        let mut devices = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/class") {
            for entry in entries.flatten() {
                let class_name = entry.file_name().to_string_lossy().into_owned();
                if let Ok(dev_entries) = fs::read_dir(entry.path()) {
                    for dev_entry in dev_entries.flatten() {
                        devices.push(Device {
                            name: dev_entry.file_name().to_string_lossy().into_owned(),
                            class: class_name.clone(),
                            path: dev_entry.path().to_string_lossy().into_owned(),
                        });
                    }
                }
            }
        }
        Ok(devices)
    }
}
