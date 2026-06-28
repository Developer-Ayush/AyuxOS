use super::common::HalResult;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait Clock {
    fn get_time(&self) -> HalResult<u64>;
}

pub struct LinuxClock;

impl Clock for LinuxClock {
    fn get_time(&self) -> HalResult<u64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|e| super::common::HalError::Internal(e.to_string()))
    }
}
