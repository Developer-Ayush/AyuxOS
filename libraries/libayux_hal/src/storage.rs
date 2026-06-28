use super::common::HalResult;
use std::fs;

pub struct StorageDevice {
    pub name: String,
    pub size: u64,
    pub removable: bool,
}

pub trait Storage {
    fn list_devices(&self) -> HalResult<Vec<StorageDevice>>;
}

pub struct LinuxStorage;

impl Storage for LinuxStorage {
    fn list_devices(&self) -> HalResult<Vec<StorageDevice>> {
        let mut devices = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/block") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("loop") || name.starts_with("ram") {
                    continue;
                }

                let size_path = entry.path().join("size");
                let size = fs::read_to_string(size_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .unwrap_or(0) * 512;

                let removable_path = entry.path().join("removable");
                let removable = fs::read_to_string(removable_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .unwrap_or(0) == 1;

                devices.push(StorageDevice {
                    name,
                    size,
                    removable,
                });
            }
        }
        Ok(devices)
    }
}
