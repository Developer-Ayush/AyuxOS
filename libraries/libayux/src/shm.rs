use std::ptr;
use nix::libc;
use std::io;

pub struct SharedMemory {
    _name: String,
    size: usize,
    ptr: *mut libc::c_void,
}

unsafe impl Send for SharedMemory {}
unsafe impl Sync for SharedMemory {}

impl SharedMemory {
    pub fn create(name: &str, size: usize) -> io::Result<Self> {
        let name_cstr = std::ffi::CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let fd = unsafe {
            libc::shm_open(
                name_cstr.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
                0o666,
            )
        };
        if fd == -1 {
            return Err(io::Error::last_os_error());
        }

        if unsafe { libc::ftruncate(fd, size as libc::off_t) } == -1 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        unsafe { libc::close(fd) };

        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            _name: name.to_string(),
            size,
            ptr,
        })
    }

    pub fn open(name: &str, size: usize) -> io::Result<Self> {
        let name_cstr = std::ffi::CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let fd = unsafe {
            libc::shm_open(
                name_cstr.as_ptr(),
                libc::O_RDWR,
                0o666,
            )
        };
        if fd == -1 {
            return Err(io::Error::last_os_error());
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        unsafe { libc::close(fd) };

        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            _name: name.to_string(),
            size,
            ptr,
        })
    }

    pub fn set_ready(&mut self, ready: bool) {
        unsafe {
            let header_ptr = self.ptr as *mut ShmHeader;
            (*header_ptr).ready = if ready { 1 } else { 0 };
            (*header_ptr).generation = (*header_ptr).generation.wrapping_add(1);
        }
    }

    pub fn is_ready(&self) -> bool {
        unsafe {
            let header_ptr = self.ptr as *const ShmHeader;
            (*header_ptr).ready != 0
        }
    }

    pub fn get_generation(&self) -> u32 {
        unsafe {
            let header_ptr = self.ptr as *const ShmHeader;
            (*header_ptr).generation
        }
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            let data_ptr = (self.ptr as *mut u8).add(std::mem::size_of::<ShmHeader>());
            let data_size = self.size - std::mem::size_of::<ShmHeader>();
            std::slice::from_raw_parts_mut(data_ptr, data_size)
        }
    }

    pub fn data_ptr(&self) -> *const u8 {
        unsafe { (self.ptr as *const u8).add(std::mem::size_of::<ShmHeader>()) }
    }
}

#[repr(C)]
struct ShmHeader {
    ready: u32,
    generation: u32,
    _reserved: [u32; 6],
}

pub const SHM_HEADER_SIZE: usize = std::mem::size_of::<ShmHeader>();

impl Drop for SharedMemory {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr, self.size);
        }
    }
}

pub fn shm_unlink(name: &str) -> io::Result<()> {
    let name_cstr = std::ffi::CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    if unsafe { libc::shm_unlink(name_cstr.as_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
