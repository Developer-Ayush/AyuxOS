use libaipc::{AipcClient, AipcMessage, LogLevel, SecurityRequest, SecurityResponse};
use libayux::ayux_log;
use std::env;
use std::io::{self, Write};

const SECURITY_SOCKET_PATH: &str = "/run/security.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

struct Shell {
    username: String,
    token: String,
    cwd: String,
    history: Vec<String>,
}

impl Shell {
    fn new() -> Self {
        let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let token = env::var("AYUX_SESSION_TOKEN").unwrap_or_else(|_| "none".to_string());
        let cwd = if username == "root" {
            "/root".to_string()
        } else {
            format!("/users/{}", username)
        };

        Self {
            username,
            token,
            cwd,
            history: Vec::new(),
        }
    }

    fn run(&mut self) {
        ayux_log(
            LogLevel::Info,
            "ayux_shell",
            &format!("Shell started for user: {}", self.username),
        );

        println!("========================================");
        println!("Ayux Shell");
        println!("====================");
        println!("Logged in as: {}", self.username);
        println!("Type 'help' for available commands.\n");

        loop {
            print!("{}@ayux:{}$ ", self.username, self.cwd);
            io::stdout().flush().ok();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() || input.is_empty() {
                break; // EOF or error
            }

            let input_trimmed = input.trim();
            if input_trimmed.is_empty() {
                continue;
            }

            // Simple history
            if self.history.last() != Some(&input_trimmed.to_string()) {
                self.history.push(input_trimmed.to_string());
            }

            let parts: Vec<&str> = input_trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "exit" | "logout" => {
                    ayux_log(
                        LogLevel::Info,
                        "ayux_shell",
                        &format!("Shell exiting for user: {}", self.username),
                    );
                    self.logout();
                    break;
                }
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
                }
                "mkdir" => {
                    if let Some(path) = parts.get(1) {
                        self.mkdir(path);
                    } else {
                        println!("Usage: mkdir <dir>");
                    }
                }
                "touch" => {
                    if let Some(path) = parts.get(1) {
                        self.touch(path);
                    } else {
                        println!("Usage: touch <file>");
                    }
                }
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
                }
                "reboot" => self.reboot(),
                "shutdown" => self.shutdown(),
                "history" => {
                    for (i, cmd) in self.history.iter().enumerate() {
                        println!("{:4}  {}", i + 1, cmd);
                    }
                }
                cmd => {
                    println!("Unknown command: '{}'", cmd);
                    self.suggest_command(cmd);
                }
            }
        }
    }

    fn print_help(&self) {
        println!("\nAyux Shell Commands:");
        println!("  help            Display this help message");
        println!("  exit, logout    End the current session");
        println!("  pwd             Print current working directory");
        println!("  cd <dir>        Change current working directory");
        println!("  ls [path]       List directory contents");
        println!("  cat <file>      Display file contents");
        println!("  mkdir <dir>     Create a new directory");
        println!("  touch <file>    Create a new empty file");
        println!("  echo [text]     Display a line of text");
        println!("  clear           Clear the terminal screen");
        println!("  whoami          Display current user");
        println!("  hostname        Display system hostname");
        println!("  date            Display current system time");
        println!("  reboot          Restart the system");
        println!("  shutdown        Shut down the system");
        println!("  history         Show command history\n");
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
        } else if path == "." || path.is_empty() {
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
            self.cwd = if self.username == "root" {
                "/root".to_string()
            } else {
                format!("/users/{}", self.username)
            };
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
            Err(e) => {
                println!("Error connecting to security manager: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Security(SecurityRequest::FsLs { path: full_path }),
        );

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::FsEntries(entries))) => {
                for entry in entries {
                    println!("{}", entry);
                }
            }
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn cat(&self, path: &str) {
        let full_path = self.resolve_path(path);
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to security manager: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Security(SecurityRequest::FsRead { path: full_path }),
        );

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::FsContent(content))) => {
                let _ = io::stdout().write_all(&content);
                println!();
            }
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn mkdir(&self, path: &str) {
        let full_path = self.resolve_path(path);
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to security manager: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Security(SecurityRequest::FsMkdir { path: full_path }),
        );

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::Success)) => {}
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn touch(&self, path: &str) {
        let full_path = self.resolve_path(path);
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to security manager: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Security(SecurityRequest::FsTouch { path: full_path }),
        );

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::Success)) => {}
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            Ok(AipcMessage::SecurityRes(SecurityResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn logout(&self) {
        use libaipc::SessionRequest;
        let mut client = match AipcClient::connect(SESSION_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to session manager: {}", e);
                return;
            }
        };

        let _ = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Session(SessionRequest::DestroySession {
                token: self.token.clone(),
            }),
        );
    }

    fn reboot(&self) {
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to security manager: {}", e);
                return;
            }
        };

        let _ = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Security(SecurityRequest::PowerReboot),
        );
    }

    fn shutdown(&self) {
        let mut client = match AipcClient::connect(SECURITY_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to security manager: {}", e);
                return;
            }
        };

        let _ = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Security(SecurityRequest::PowerShutdown),
        );
    }

    fn suggest_command(&self, cmd: &str) {
        let commands = [
            "help", "exit", "logout", "pwd", "cd", "ls", "cat", "mkdir", "touch", "echo", "clear",
            "whoami", "hostname", "date", "reboot", "shutdown", "history",
        ];

        let mut suggestions = Vec::new();
        for &c in &commands {
            if self.levenshtein_distance(cmd, c) <= 2 {
                suggestions.push(c);
            }
        }

        if !suggestions.is_empty() {
            println!("Did you mean one of these?");
            for s in suggestions {
                println!("  {}", s);
            }
        }
    }

    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let v1: Vec<char> = s1.chars().collect();
        let v2: Vec<char> = s2.chars().collect();
        let n = v1.len();
        let m = v2.len();

        let mut dp = vec![vec![0; m + 1]; n + 1];

        for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
            row[0] = i;
        }
        for (j, val) in dp[0].iter_mut().enumerate().take(m + 1) {
            *val = j;
        }

        for i in 1..=n {
            for j in 1..=m {
                let cost = if v1[i - 1] == v2[j - 1] { 0 } else { 1 };
                dp[i][j] = std::cmp::min(
                    dp[i - 1][j] + 1,
                    std::cmp::min(dp[i][j - 1] + 1, dp[i - 1][j - 1] + cost),
                );
            }
        }

        dp[n][m]
    }
}

fn main() {
    let mut shell = Shell::new();
    shell.run();
}
