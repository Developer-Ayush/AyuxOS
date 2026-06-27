use std::io::{self, Write};
use std::env;
use std::process::exit;
use std::fs;
use std::path::Path;
use nix::sys::reboot::{reboot, RebootMode};

fn main() {
    let user = env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    loop {
        let cwd = env::current_dir().unwrap_or_else(|_| Path::new("/").to_path_buf());
        print!("{}@{}:{}# ", user, "ayux", cwd.display());
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
            "clear" => print!("{}[2J{}[1;1H", 27 as char, 27 as char),
            "pwd" => println!("{}", cwd.display()),
            "ls" => ls(args),
            "cd" => cd(args),
            "mkdir" => mkdir(args),
            "touch" => touch(args),
            "cat" => cat(args),
            "echo" => println!("{}", args.join(" ")),
            "whoami" => println!("{}", user),
            "logout" => break,
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
            "exit" => break,
            _ => {
                println!("ayux_shell: command not found: {}", cmd);
            }
        }
    }
}

fn help() {
    println!("AyuxOS Shell - Milestone 1");
    println!("Built-in commands: help, clear, pwd, ls, cd, mkdir, touch, cat, echo, whoami, logout, reboot, shutdown, exit");
}

fn ls(args: &[&str]) {
    let path = if args.is_empty() { "." } else { args[0] };
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("{}", entry.file_name().to_string_lossy());
                }
            }
        },
        Err(e) => println!("ls: {}: {}", path, e),
    }
}

fn cd(args: &[&str]) {
    let new_dir = if args.is_empty() { "/" } else { args[0] };
    if let Err(e) = env::set_current_dir(new_dir) {
        println!("cd: {}: {}", new_dir, e);
    }
}

fn mkdir(args: &[&str]) {
    if args.is_empty() {
        println!("mkdir: missing operand");
        return;
    }
    for arg in args {
        if let Err(e) = fs::create_dir(arg) {
            println!("mkdir: cannot create directory '{}': {}", arg, e);
        }
    }
}

fn touch(args: &[&str]) {
    if args.is_empty() {
        println!("touch: missing file operand");
        return;
    }
    for arg in args {
        if let Err(e) = fs::OpenOptions::new().create(true).write(true).open(arg) {
            println!("touch: cannot touch '{}': {}", arg, e);
        }
    }
}

fn cat(args: &[&str]) {
    if args.is_empty() {
        return;
    }
    for arg in args {
        match fs::read_to_string(arg) {
            Ok(content) => print!("{}", content),
            Err(e) => println!("cat: {}: {}", arg, e),
        }
    }
    println!();
}
