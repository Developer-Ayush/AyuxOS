use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

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
    libayux::print_heading("AyuxOS - Graphical Foundation");
    println!("AyuxOS Booting...\n");

    if let Err(e) = libayux::mount_basic_filesystems() {
        eprintln!("[Ayux Init] ERROR: Failed to mount filesystems: {}", e);
    }

    libayux::setup_env();

    let config_path = "/etc/ayux_services.toml";
    let config_str = fs::read_to_string(config_path).unwrap_or_else(|_| {
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

        [services.window_server]
        path = "/bin/window_server"
        dependencies = ["log_service"]
        restart_policy = "always"
        priority = 4
        health_check_socket = "/run/window_server.sock"
        "#
        .to_string()
    });

    let config: Config = toml::from_str(&config_str).expect("Failed to parse service config");

    let mut services: HashMap<String, ServiceInfo> = config
        .services
        .into_iter()
        .map(|(name, cfg)| {
            (
                name,
                ServiceInfo {
                    config: cfg,
                    child: None,
                    consecutive_failures: 0,
                },
            )
        })
        .collect();

    let mut to_start: Vec<String> = services.keys().cloned().collect();
    to_start.sort_by_key(|name| services[name].config.priority);

    for name in to_start {
        let display_name = match name.as_str() {
            "log_service" => "Log Service",
            "auth_service" => "Authentication Service",
            "session_manager" => "Session Manager",
            "security_manager" => "Security Manager",
            "network_manager" => "Network Manager",
            "window_server" => "Window Server & Compositor",
            _ => &name,
        };
        print!(" • Starting {:<30}", display_name);
        io::stdout().flush().ok();
        let path = services[&name].config.path.clone();
        match Command::new(&path).spawn() {
            Ok(child) => {
                if let Some(info) = services.get_mut(&name) {
                    info.child = Some(child);
                }
                println!("OK");
            }
            Err(e) => {
                println!("FAILED");
                eprintln!("[Ayux Init] Failed to start {}: {}", name, e);
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    print!(" • Starting {:<30}", "Login Manager (GUI)");
    io::stdout().flush().ok();
    let mut login_manager: Option<Child> = None;
    let mut desktop_started = false;

    loop {
        for (name, info) in services.iter_mut() {
            let mut restart_needed = false;
            if let Some(child) = info.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!("[Ayux Init] Service {} exited with status: {}.", name, status);
                        restart_needed = true;
                    }
                    Ok(None) => {
                        if let Some(socket_path) = &info.config.health_check_socket {
                            if !Path::new(socket_path).exists() {
                                info.consecutive_failures += 1;
                                if info.consecutive_failures >= 5 {
                                    println!("[Ayux Init] Service {} is unhealthy. Restarting...", name);
                                    let _ = child.kill();
                                    restart_needed = true;
                                    info.consecutive_failures = 0;
                                }
                            } else {
                                info.consecutive_failures = 0;
                            }
                        }
                    }
                    Err(_) => restart_needed = true,
                }
            } else {
                restart_needed = true;
            }

            if restart_needed && info.config.restart_policy == "always" {
                match Command::new(&info.config.path).spawn() {
                    Ok(new_child) => info.child = Some(new_child),
                    Err(_) => {}
                }
            }
        }

        let login_manager_needs_start = match login_manager.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => !desktop_started,
                Ok(None) => false,
                Err(_) => !desktop_started,
            },
            None => !desktop_started,
        };

        if login_manager_needs_start {
            match Command::new("/bin/login_manager_gui").spawn() {
                Ok(child) => {
                    if login_manager.is_none() {
                        println!("OK");
                        println!("\nSystem Ready.\n");
                    }
                    login_manager = Some(child);
                }
                Err(_) => {
                    if login_manager.is_none() {
                        println!("FAILED");
                    }
                }
            }
        }

        if !desktop_started {
            let output = Command::new("pgrep").arg("-x").arg("desktop").output();
            if let Ok(out) = output {
                if out.status.success() {
                    desktop_started = true;
                }
            }
        }

        thread::sleep(Duration::from_secs(2));
    }
}
