use std::io::{self, Write};
use std::env;
use std::process::exit;
use std::path::{Path, PathBuf};
use nix::sys::reboot::{reboot, RebootMode};
use libaipc::{AipcClient, AipcMessage, SecurityRequest, SecurityResponse};

const SECURITY_SOCKET_PATH: &str = "/run/security.sock";

fn main() {
    let user = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let token = env::var("AYUX_SESSION_TOKEN").unwrap_or_else(|_| "none".to_string());
    let mut current_dir = PathBuf::from("/");

    if user != "root" {
        current_dir = PathBuf::from(format!("/users/{}", user));
    }

    loop {
        print!("{}@ayux:{}# ", user, current_dir.display());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).unwrap() == 0 {
            break; // EOF
        }

        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "help" => help(),
            "pwd" => println!("{}", current_dir.display()),
            "ls" => ls(&token, &current_dir, args),
            "cd" => {
                if let Some(new_dir) = cd(&token, &current_dir, args) {
                    current_dir = new_dir;
                }
            },
            "mkdir" => mkdir(&token, &current_dir, args),
            "touch" => touch(&token, &current_dir, args),
            "cat" => cat(&token, &current_dir, args),
            "echo" => println!("{}", args.join(" ")),
            "whoami" => println!("{}", user),
            "logout" | "exit" => break,
            "reboot" => {
                println!("Rebooting...");
                let _ = reboot(RebootMode::RB_AUTOBOOT);
                exit(0);
            },
            "shutdown" => {
                println!("Shutting down...");
                let _ = reboot(RebootMode::RB_POWER_OFF);
                exit(0);
            },
            _ => println!("ayux_shell: command not found: {}", cmd),
        }
    }
}

fn help() {
    println!("AyuxOS Shell - Milestone 2 (Security Enforcement Mode)");
    println!("Commands are now authorized by the Security Manager via AIPC.");
    println!("Available commands: help, pwd, ls, cd, mkdir, touch, cat, echo, whoami, logout, reboot, shutdown, exit");
}

fn get_abs_path(current: &Path, arg: &str) -> String {
    if arg.starts_with('/') {
        arg.to_string()
    } else {
        let mut path = current.to_path_buf();
        path.push(arg);
        path.to_string_lossy().to_string()
    }
}

fn ls(token: &str, current: &Path, args: &[&str]) {
    let path = if args.is_empty() { current.to_string_lossy().to_string() } else { get_abs_path(current, args[0]) };

    let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
        Ok(c) => c,
        Err(e) => { println!("Error connecting to security manager: {}", e); return; }
    };

    let _ = client.send_message(&AipcMessage::Security(SecurityRequest::FsLs {
        token: token.to_string(),
        path,
    }));

    match client.receive_message() {
        Ok(AipcMessage::SecurityRes(SecurityResponse::FsEntries(entries))) => {
            for entry in entries {
                println!("{}", entry);
            }
        },
        Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Permission denied: {}", e),
        Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("ls: {}", e),
        _ => println!("ls: Received unexpected response"),
    }
}

fn cd(token: &str, current: &Path, args: &[&str]) -> Option<PathBuf> {
    let path = if args.is_empty() {
        if env::var("USER").unwrap_or_default() == "root" { "/root".to_string() }
        else { format!("/users/{}", env::var("USER").unwrap_or_default()) }
    } else {
        get_abs_path(current, args[0])
    };

    let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
        Ok(c) => c,
        Err(e) => { println!("Error connecting to security manager: {}", e); return None; }
    };

    let _ = client.send_message(&AipcMessage::Security(SecurityRequest::Authorize {
        token: token.to_string(),
        operation: "cd".to_string(),
        path: path.clone(),
    }));

    match client.receive_message() {
        Ok(AipcMessage::SecurityRes(SecurityResponse::Allowed)) => Some(PathBuf::from(path)),
        Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => { println!("Permission denied: {}", e); None },
        _ => { println!("cd: Access check failed"); None }
    }
}

fn mkdir(token: &str, current: &Path, args: &[&str]) {
    if args.is_empty() { println!("mkdir: missing operand"); return; }
    let path = get_abs_path(current, args[0]);

    let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
        Ok(c) => c,
        Err(e) => { println!("Error connecting to security manager: {}", e); return; }
    };

    let _ = client.send_message(&AipcMessage::Security(SecurityRequest::FsMkdir {
        token: token.to_string(),
        path,
    }));

    match client.receive_message() {
        Ok(AipcMessage::SecurityRes(SecurityResponse::Success)) => (),
        Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Permission denied: {}", e),
        Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("mkdir: {}", e),
        _ => println!("mkdir: Unexpected response"),
    }
}

fn touch(token: &str, current: &Path, args: &[&str]) {
    if args.is_empty() { println!("touch: missing operand"); return; }
    let path = get_abs_path(current, args[0]);

    let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
        Ok(c) => c,
        Err(e) => { println!("Error connecting to security manager: {}", e); return; }
    };

    let _ = client.send_message(&AipcMessage::Security(SecurityRequest::FsTouch {
        token: token.to_string(),
        path,
    }));

    match client.receive_message() {
        Ok(AipcMessage::SecurityRes(SecurityResponse::Success)) => (),
        Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Permission denied: {}", e),
        Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("touch: {}", e),
        _ => println!("touch: Unexpected response"),
    }
}

fn cat(token: &str, current: &Path, args: &[&str]) {
    if args.is_empty() { return; }
    let path = get_abs_path(current, args[0]);

    let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
        Ok(c) => c,
        Err(e) => { println!("Error connecting to security manager: {}", e); return; }
    };

    let _ = client.send_message(&AipcMessage::Security(SecurityRequest::FsRead {
        token: token.to_string(),
        path,
    }));

    match client.receive_message() {
        Ok(AipcMessage::SecurityRes(SecurityResponse::FsContent(content))) => {
            println!("{}", String::from_utf8_lossy(&content));
        },
        Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Permission denied: {}", e),
        Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("cat: {}", e),
        _ => println!("cat: Unexpected response"),
    }
}
