use nix::mount::{mount, MsFlags};
use std::fs;
use std::io;
use libaipc::{AipcClient, AipcMessage, LogRequest, LogLevel};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn ayux_log(level: LogLevel, module: &str, message: &str) {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let msg = AipcMessage::Log(LogRequest::Log {
        level: level.clone(),
        module: module.to_string(),
        message: message.to_string(),
        timestamp: ts,
    });

    if let Ok(mut client) = AipcClient::connect("/run/log.sock") {
        let _ = client.send_envelope(&libaipc::AipcEnvelope {
            header: libaipc::AipcHeader {
                version: libaipc::AIPC_VERSION,
                message_type: libaipc::MessageType::Request,
                sender: "unknown".to_string(), // In a real app, this would be set correctly
                session_id: None,
                correlation_id: 0,
            },
            message: msg,
        });
    } else {
        // Fallback to stderr if log service is unavailable
        eprintln!("[{}] [{:?}] [{}] {}", ts, level, module, message);
    }
}

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

    fs::create_dir_all("/main")?;
    fs::create_dir_all("/root")?;
    fs::create_dir_all("/users")?;

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
