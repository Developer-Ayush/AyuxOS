use nix::mount::{mount, MsFlags};
use std::fs;
use std::io;

pub fn mount_basic_filesystems() -> io::Result<()> {
    println!("[Ayux Init] Mounting basic filesystems...");

    fs::create_dir_all("/proc")?;
    fs::create_dir_all("/sys")?;
    fs::create_dir_all("/dev")?;
    fs::create_dir_all("/run")?;
    fs::create_dir_all("/tmp")?;

    mount_fs("proc", "/proc", "proc", MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC)?;
    mount_fs("sysfs", "/sys", "sysfs", MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC)?;
    mount_fs("devtmpfs", "/dev", "devtmpfs", MsFlags::MS_NOSUID)?;
    mount_fs("tmpfs", "/run", "tmpfs", MsFlags::MS_NOSUID | MsFlags::MS_NODEV)?;
    mount_fs("tmpfs", "/tmp", "tmpfs", MsFlags::MS_NOSUID | MsFlags::MS_NODEV)?;

    Ok(())
}

fn mount_fs(source: &str, target: &str, fstype: &str, flags: MsFlags) -> io::Result<()> {
    mount(
        Some(source),
        target,
        Some(fstype),
        flags,
        None::<&str>,
    ).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to mount {}: {}", target, e)))
}

pub fn setup_env() {
    unsafe {
        std::env::set_var("PATH", "/bin:/usr/bin");
        std::env::set_var("TERM", "linux");
    }
}
