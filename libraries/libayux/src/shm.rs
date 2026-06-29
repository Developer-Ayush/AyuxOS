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

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.size) }
    }
}

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
