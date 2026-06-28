use super::common::HalResult;

pub struct DisplayInfo {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
}

pub trait Display {
    fn get_info(&self) -> HalResult<DisplayInfo>;
    fn flush(&mut self) -> HalResult<()>;
}

pub struct LinuxFramebuffer {}

impl LinuxFramebuffer {
    pub fn new(_path: &str) -> Self {
        Self {}
    }
}

impl Display for LinuxFramebuffer {
    fn get_info(&self) -> HalResult<DisplayInfo> {
        // In a real implementation, we would use ioctl FBIOGET_VSCREENINFO
        // For Milestone 3, we provide a reasonable default or read from sysfs if available
        Ok(DisplayInfo {
            width: 1024,
            height: 768,
            bpp: 32,
        })
    }

    fn flush(&mut self) -> HalResult<()> {
        Ok(())
    }
}
