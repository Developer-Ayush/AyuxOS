use super::common::HalResult;
use std::fs::File;
use std::io::Read;

pub trait Random {
    fn get_random(&self, buf: &mut [u8]) -> HalResult<()>;
}

pub struct LinuxRandom;

impl Random for LinuxRandom {
    fn get_random(&self, buf: &mut [u8]) -> HalResult<()> {
        let mut file = File::open("/dev/urandom")?;
        file.read_exact(buf)?;
        Ok(())
    }
}
