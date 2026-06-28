use super::common::HalResult;

pub trait Power {
    fn reboot(&self) -> HalResult<()>;
    fn shutdown(&self) -> HalResult<()>;
}

pub struct LinuxPower;

impl Power for LinuxPower {
    fn reboot(&self) -> HalResult<()> {
        // In a real system, this would use reboot(RB_AUTOBOOT)
        Ok(())
    }

    fn shutdown(&self) -> HalResult<()> {
        // In a real system, this would use reboot(RB_POWER_OFF)
        Ok(())
    }
}
