use libayux;
use std::process::{Command, Child};
use std::thread;
use std::time::Duration;
use serde::Deserialize;
use std::fs;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct ServiceConfig {
    path: String,
    #[allow(dead_code)]
    dependencies: Vec<String>,
    restart_policy: String,
    priority: u32,
    health_check_socket: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Config {
    services: HashMap<String, ServiceConfig>,
}

struct ServiceInfo {
    config: ServiceConfig,
    child: Option<Child>,
    consecutive_failures: u32,
}

fn main() {
    println!("--- AyuxOS Starting (Milestone 3) ---");

    if let Err(e) = libayux::mount_basic_filesystems() {
        eprintln!("[Ayux Init] ERROR: Failed to mount filesystems: {}", e);
    }

    libayux::setup_env();

    let config_path = "/etc/ayux_services.toml";
    let config_str = fs::read_to_string(config_path).unwrap_or_else(|_| {
        eprintln!("[Ayux Init] ERROR: Could not read {}, using hardcoded fallback", config_path);
        r#"
        [services.log_service]
        path = "/bin/log_service"
        dependencies = []
        restart_policy = "always"
        priority = 1
        health_check_socket = "/run/log.sock"

        [services.auth_service]
        path = "/bin/auth_service"
        dependencies = ["log_service"]
        restart_policy = "always"
        priority = 2
        health_check_socket = "/run/auth.sock"

        [services.session_manager]
        path = "/bin/session_manager"
        dependencies = ["log_service"]
        restart_policy = "always"
        priority = 2
        health_check_socket = "/run/session.sock"

        [services.security_manager]
        path = "/bin/security_manager"
        dependencies = ["session_manager", "log_service"]
        restart_policy = "always"
        priority = 3
        health_check_socket = "/run/security.sock"

        [services.network_manager]
        path = "/bin/network_manager"
        dependencies = ["log_service"]
        restart_policy = "always"
        priority = 3
        health_check_socket = "/run/network.sock"
        "#.to_string()
    });

    let config: Config = toml::from_str(&config_str).expect("Failed to parse service config");

    let mut services: HashMap<String, ServiceInfo> = config.services.into_iter().map(|(name, cfg)| {
        (name, ServiceInfo { config: cfg, child: None, consecutive_failures: 0 })
    }).collect();

    // Startup based on priority
    let mut to_start: Vec<String> = services.keys().cloned().collect();
    to_start.sort_by_key(|name| services[name].config.priority);

    println!("[Ayux Init] Starting System Services...");

    for name in to_start {
        println!("[Ayux Init] Starting {}...", name);
        let path = services[&name].config.path.clone();
        match Command::new(&path).spawn() {
            Ok(child) => {
                services.get_mut(&name).unwrap().child = Some(child);
            },
            Err(e) => eprintln!("[Ayux Init] Failed to start {}: {}", name, e),
        }
        thread::sleep(Duration::from_millis(500));
    }

    println!("[Ayux Init] Starting Login Manager...");
    let mut login_manager: Option<Child> = None;

    loop {
        // Monitor services
        for (name, info) in services.iter_mut() {
            let mut restart_needed = false;

            if let Some(child) = info.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!("[Ayux Init] Service {} exited with status: {}.", name, status);
                        restart_needed = true;
                    },
                    Ok(None) => {
                        // Check health if specified
                        if let Some(socket_path) = &info.config.health_check_socket {
                            if !Path::new(socket_path).exists() {
                                info.consecutive_failures += 1;
                                if info.consecutive_failures >= 5 {
                                    println!("[Ayux Init] Service {} is unhealthy (socket {} missing). Killing and restarting...", name, socket_path);
                                    let _ = child.kill();
                                    restart_needed = true;
                                    info.consecutive_failures = 0;
                                }
                            } else {
                                info.consecutive_failures = 0;
                            }
                        }
                    },
                    Err(e) => eprintln!("[Ayux Init] Error monitoring {}: {}", name, e),
                }
            } else {
                restart_needed = true;
            }

            if restart_needed && info.config.restart_policy == "always" {
                println!("[Ayux Init] Restarting {}...", name);
                match Command::new(&info.config.path).spawn() {
                    Ok(new_child) => info.child = Some(new_child),
                    Err(e) => eprintln!("[Ayux Init] Failed to restart {}: {}", name, e),
                }
            }
        }

        // Manage Login Manager
        let login_manager_needs_start = match login_manager.as_mut() {
            Some(child) => {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!("[Ayux Init] Login manager exited with status: {}. Restarting...", status);
                        true
                    },
                    Ok(None) => false,
                    Err(e) => {
                        eprintln!("[Ayux Init] Error monitoring login manager: {}", e);
                        true
                    }
                }
            },
            None => true,
        };

        if login_manager_needs_start {
            match Command::new("/bin/login_manager").spawn() {
                Ok(child) => login_manager = Some(child),
                Err(e) => eprintln!("[Ayux Init] Failed to start login manager: {}", e),
            }
        }

        thread::sleep(Duration::from_secs(2));
    }
}
