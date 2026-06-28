use libaipc::{AipcClient, AipcMessage, SecurityRequest, SecurityResponse, LogLevel};
use std::io::{self, Write};
use libayux::ayux_log;
use std::env;

const SECURITY_SOCKET_PATH: &str = "/run/security.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

struct Shell {
    username: String,
    token: String,
    cwd: String,
}

impl Shell {
    fn new() -> Self {
        let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let token = env::var("AYUX_SESSION_TOKEN").unwrap_or_else(|_| "none".to_string());
        let cwd = if username == "root" { "/root".to_string() } else { format!("/users/{}", username) };

        Self {
            username,
            token,
            cwd,
        }
    }

    fn run(&mut self) {
        ayux_log(LogLevel::Info, "ayux_shell", &format!("Shell started for user: {}", self.username));
        println!("Ayux Shell v0.1");
        println!("Logged in as: {}", self.username);

        loop {
            print!("{}@{}:{}$ ", self.username, "ayux", self.cwd);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).unwrap() == 0 {
                break; // EOF
            }

            let parts: Vec<&str> = input.trim().split_whitespace().collect();
            if parts.is_empty() { continue; }

            match parts[0] {
                "exit" | "logout" => {
                    ayux_log(LogLevel::Info, "ayux_shell", &format!("Shell exiting for user: {}", self.username));
                    self.logout();
                    break;
                },
                "help" => self.print_help(),
                "pwd" => println!("{}", self.cwd),
                "cd" => self.cd(parts.get(1).cloned().unwrap_or("")),
                "ls" => self.ls(parts.get(1).cloned().unwrap_or(".")),
                "cat" => {
                    if let Some(path) = parts.get(1) {
                        self.cat(path);
                    } else {
                        println!("Usage: cat <file>");
                    }
                },
                "mkdir" => {
                    if let Some(path) = parts.get(1) {
                        self.mkdir(path);
                    } else {
                        println!("Usage: mkdir <dir>");
                    }
                },
                "touch" => {
                    if let Some(path) = parts.get(1) {
                        self.touch(path);
                    } else {
                        println!("Usage: touch <file>");
                    }
                },
                "echo" => println!("{}", parts[1..].join(" ")),
                "clear" => print!("\x1B[2J\x1B[1;1H"),
                "whoami" => println!("{}", self.username),
                "hostname" => println!("ayux"),
                "date" => {
                    use std::time::SystemTime;
                    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                        Ok(n) => println!("{}", n.as_secs()),
                        Err(_) => println!("Error getting time"),
                    }
                },
                "reboot" => self.reboot(),
                "shutdown" => self.shutdown(),
                cmd => println!("Unknown command: {}", cmd),
            }
        }
    }

    fn print_help(&self) {
        println!("Available commands: help, exit, logout, pwd, cd, ls, cat, mkdir, touch, echo, clear, whoami, hostname, date, reboot, shutdown");
    }

    fn resolve_path(&self, path: &str) -> String {
        if path.starts_with('/') {
            path.to_string()
        } else if path == ".." {
            let mut p = std::path::PathBuf::from(&self.cwd);
            if p.pop() {
                p.to_string_lossy().to_string()
            } else {
                "/".to_string()
            }
        } else if path == "." || path == "" {
            self.cwd.clone()
        } else {
            let mut base = self.cwd.clone();
            if !base.ends_with('/') {
                base.push('/');
            }
            base.push_str(path);
            base
        }
    }

    fn cd(&mut self, path: &str) {
        if path.is_empty() {
            self.cwd = if self.username == "root" { "/root".to_string() } else { format!("/users/{}", self.username) };
            return;
        }
        let new_path = self.resolve_path(path);
        // We should check if it exists and is a directory via Security Manager
        // For simplicity in this milestone, we just update the CWD
        self.cwd = new_path;
    }

    fn ls(&self, path: &str) {
        let full_path = self.resolve_path(path);
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => { println!("Error connecting to security manager: {}", e); return; }
        };

        let res = client.request("ayux_shell", Some(self.token.clone()), AipcMessage::Security(SecurityRequest::FsLs {
            path: full_path,
        }));

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::FsEntries(entries))) => {
                for entry in entries {
                    println!("{}", entry);
                }
            },
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn cat(&self, path: &str) {
        let full_path = self.resolve_path(path);
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => { println!("Error connecting to security manager: {}", e); return; }
        };

        let res = client.request("ayux_shell", Some(self.token.clone()), AipcMessage::Security(SecurityRequest::FsRead {
            path: full_path,
        }));

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::FsContent(content))) => {
                io::stdout().write_all(&content).unwrap();
                println!();
            },
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn mkdir(&self, path: &str) {
        let full_path = self.resolve_path(path);
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => { println!("Error connecting to security manager: {}", e); return; }
        };

        let res = client.request("ayux_shell", Some(self.token.clone()), AipcMessage::Security(SecurityRequest::FsMkdir {
            path: full_path,
        }));

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::Success)) => {},
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn touch(&self, path: &str) {
        let full_path = self.resolve_path(path);
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => { println!("Error connecting to security manager: {}", e); return; }
        };

        let res = client.request("ayux_shell", Some(self.token.clone()), AipcMessage::Security(SecurityRequest::FsTouch {
            path: full_path,
        }));

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::Success)) => {},
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn logout(&self) {
        use libaipc::SessionRequest;
        let mut client = match AipcClient::connect(SESSION_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => { println!("Error connecting to session manager: {}", e); return; }
        };

        let _ = client.request("ayux_shell", Some(self.token.clone()), AipcMessage::Session(SessionRequest::DestroySession {
            token: self.token.clone(),
        }));
    }

    fn reboot(&self) {
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => { println!("Error connecting to security manager: {}", e); return; }
        };

        let _ = client.request("ayux_shell", Some(self.token.clone()), AipcMessage::Security(SecurityRequest::PowerReboot));
    }

    fn shutdown(&self) {
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => { println!("Error connecting to security manager: {}", e); return; }
        };

        let _ = client.request("ayux_shell", Some(self.token.clone()), AipcMessage::Security(SecurityRequest::PowerShutdown));
    }
}

fn main() {
    let mut shell = Shell::new();
    shell.run();
}
