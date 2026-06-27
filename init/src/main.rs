use libayux;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("--- AyuxOS Starting (Milestone 2) ---");

    if let Err(e) = libayux::mount_basic_filesystems() {
        eprintln!("[Ayux Init] ERROR: Failed to mount filesystems: {}", e);
    }

    libayux::setup_env();

    println!("[Ayux Init] Starting System Services...");

    let services = [
        ("/bin/auth_service", "Auth Service"),
        ("/bin/session_manager", "Session Manager"),
        ("/bin/security_manager", "Security Manager"),
    ];

    for (path, name) in services {
        println!("[Ayux Init] Starting {}...", name);
        Command::new(path)
            .spawn()
            .expect(&format!("Failed to start {}", name));
    }

    // Give services a moment to start and create their sockets
    thread::sleep(Duration::from_millis(500));

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
