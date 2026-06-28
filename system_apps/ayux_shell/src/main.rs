use libaipc::{AipcClient, AipcMessage, SecurityRequest, SecurityResponse};
use std::io::{self, Write};
use std::env;

const SECURITY_SOCKET_PATH: &str = "/run/security.sock";

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
                "exit" => break,
                "help" => self.print_help(),
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
                "whoami" => println!("{}", self.username),
                cmd => println!("Unknown command: {}", cmd),
            }
        }
    }

    fn print_help(&self) {
        println!("Available commands: help, exit, ls, cat, mkdir, touch, whoami");
    }

    fn resolve_path(&self, path: &str) -> String {
        if path.starts_with('/') {
            path.to_string()
        } else {
            let mut base = self.cwd.clone();
            if !base.ends_with('/') {
                base.push('/');
            }
            base.push_str(path);
            base
        }
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
}

fn main() {
    let mut shell = Shell::new();
    shell.run();
}
