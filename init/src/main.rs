use libayux;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("--- AyuxOS Starting ---");

    if let Err(e) = libayux::mount_basic_filesystems() {
        eprintln!("[Ayux Init] ERROR: Failed to mount filesystems: {}", e);
    }

    libayux::setup_env();

    println!("[Ayux Init] Starting Login Manager...");

    loop {
        let mut child = Command::new("/bin/login_manager")
            .spawn()
            .expect("Failed to start login manager");

        let status = child.wait().expect("Login manager crashed");
        println!("[Ayux Init] Login manager exited with status: {}. Restarting...", status);
        thread::sleep(Duration::from_secs(1));
    }
}
