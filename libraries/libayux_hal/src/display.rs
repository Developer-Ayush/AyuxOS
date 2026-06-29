use super::common::{HalError, HalResult};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::ptr;
use nix::libc;

#[repr(C)]
pub struct fb_bitfield {
    pub offset: u32,
    pub length: u32,
    pub msb_right: u32,
}

#[repr(C)]
pub struct fb_var_screeninfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: fb_bitfield,
    pub green: fb_bitfield,
    pub blue: fb_bitfield,
    pub transp: fb_bitfield,
    pub nonstd: u32,
    pub activate: u32,
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

#[repr(C)]
pub struct fb_fix_screeninfo {
    pub id: [u8; 16],
    pub smem_start: usize,
    pub smem_len: u32,
    pub type_: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: usize,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelFormat {
    Rgb565,
    Rgb888,
    Bgr888,
    Xrgb8888,
    Argb8888,
    Bgrx8888,
    Bgra8888,
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayInfo {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    pub pitch: u32,
    pub format: PixelFormat,
}

pub trait Display {
    fn get_info(&self) -> HalResult<DisplayInfo>;
    fn flip(&mut self) -> HalResult<()>;
    fn get_buffer(&mut self) -> &mut [u8];
}

pub struct LinuxFramebuffer {
    _file: File,
    info: DisplayInfo,
    mmap_ptr: *mut libc::c_void,
    mmap_size: usize,
    buffer: Vec<u8>, // Internal buffer (always BGRA8888)
}

unsafe impl Send for LinuxFramebuffer {}
unsafe impl Sync for LinuxFramebuffer {}

impl LinuxFramebuffer {
    pub fn new(path: &str) -> HalResult<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| HalError::IOError(e.to_string()))?;

        let fd = file.as_raw_fd();

        let (vinfo, finfo) = unsafe {
            let mut vinfo: fb_var_screeninfo = std::mem::zeroed();
            let mut finfo: fb_fix_screeninfo = std::mem::zeroed();

            if libc::ioctl(fd, 0x4600, &mut vinfo) == -1 {
                return Err(HalError::HardwareError("Failed to get fb_var_screeninfo".into()));
            }
            if libc::ioctl(fd, 0x4602, &mut finfo) == -1 {
                return Err(HalError::HardwareError("Failed to get fb_fix_screeninfo".into()));
            }
            (vinfo, finfo)
        };

        let width = vinfo.xres;
        let height = vinfo.yres;
        let bpp = vinfo.bits_per_pixel;
        let pitch = finfo.line_length;
        let mmap_size = (finfo.smem_len) as usize;

        let format = match (bpp, vinfo.red.offset, vinfo.blue.offset) {
            (16, 11, 0) => PixelFormat::Rgb565,
            (24, 16, 0) => PixelFormat::Bgr888,
            (24, 0, 16) => PixelFormat::Rgb888,
            (32, 16, 0) => PixelFormat::Bgrx8888,
            (32, 0, 16) => PixelFormat::Xrgb8888,
            _ => {
                // Fallback or more complex detection
                if bpp == 32 {
                    if vinfo.red.offset == 0 { PixelFormat::Xrgb8888 } else { PixelFormat::Bgrx8888 }
                } else if bpp == 16 {
                    PixelFormat::Rgb565
                } else {
                    PixelFormat::Bgrx8888 // Default
                }
            }
        };

        let mmap_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                mmap_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if mmap_ptr == libc::MAP_FAILED {
            return Err(HalError::IOError("mmap failed".into()));
        }

        let info = DisplayInfo {
            width,
            height,
            bpp,
            pitch,
            format,
        };

        // Internal buffer is always 32bpp BGRA8888 for libgraphics
        let buffer_size = (width * height * 4) as usize;
        let buffer = vec![0u8; buffer_size];

        Ok(Self {
            _file: file,
            info,
            mmap_ptr,
            mmap_size,
            buffer,
        })
    }
}

impl Display for LinuxFramebuffer {
    fn get_info(&self) -> HalResult<DisplayInfo> {
        Ok(self.info)
    }

    fn flip(&mut self) -> HalResult<()> {
        unsafe {
            let dest = self.mmap_ptr as *mut u8;
            let src = self.buffer.as_ptr();

            match self.info.format {
                PixelFormat::Bgrx8888 | PixelFormat::Bgra8888 => {
                    // Direct copy if formats match (Internal is BGRA8888)
                    // We need to handle pitch differences though
                    for y in 0..self.info.height {
                        let src_offset = (y * self.info.width * 4) as usize;
                        let dest_offset = (y * self.info.pitch) as usize;
                        ptr::copy_nonoverlapping(
                            src.add(src_offset),
                            dest.add(dest_offset),
                            (self.info.width * 4) as usize,
                        );
                    }
                }
                PixelFormat::Xrgb8888 | PixelFormat::Argb8888 => {
                    for y in 0..self.info.height {
                        for x in 0..self.info.width {
                            let src_offset = (y * self.info.width * 4 + x * 4) as usize;
                            let dest_offset = (y * self.info.pitch + x * 4) as usize;
                            let b = *src.add(src_offset);
                            let g = *src.add(src_offset + 1);
                            let r = *src.add(src_offset + 2);
                            let a = *src.add(src_offset + 3);

                            *dest.add(dest_offset) = r;
                            *dest.add(dest_offset + 1) = g;
                            *dest.add(dest_offset + 2) = b;
                            *dest.add(dest_offset + 3) = a;
                        }
                    }
                }
                PixelFormat::Rgb565 => {
                    for y in 0..self.info.height {
                        for x in 0..self.info.width {
                            let src_offset = (y * self.info.width * 4 + x * 4) as usize;
                            let dest_offset = (y * self.info.pitch + x * 2) as usize;
                            let b = *src.add(src_offset) as u16;
                            let g = *src.add(src_offset + 1) as u16;
                            let r = *src.add(src_offset + 2) as u16;

                            let rgb565 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
                            let dest_ptr = dest.add(dest_offset) as *mut u16;
                            *dest_ptr = rgb565;
                        }
                    }
                }
                _ => {
                    // Fallback to copy if unknown, but respect pitch
                    for y in 0..self.info.height {
                        let src_offset = (y * self.info.width * 4) as usize;
                        let dest_offset = (y * self.info.pitch) as usize;
                        let copy_len = std::cmp::min((self.info.width * 4) as usize, self.info.pitch as usize);
                        ptr::copy_nonoverlapping(
                            src.add(src_offset),
                            dest.add(dest_offset),
                            copy_len,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn get_buffer(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}

impl Drop for LinuxFramebuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mmap_ptr, self.mmap_size);
        }
    }
}
