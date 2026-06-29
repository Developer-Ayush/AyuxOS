use libaipc::{AipcClient, AipcMessage, AuthRequest, AuthResponse, LogLevel, SecurityRequest, SecurityResponse};
use libayux::ayux_log;
use libayux::paths;
use std::env;
use std::io::{self, Write};

const AUTH_SOCKET_PATH: &str = paths::AUTH_SOCKET;
const SECURITY_SOCKET_PATH: &str = paths::SECURITY_SOCKET;
const SESSION_SOCKET_PATH: &str = paths::SESSION_SOCKET;

struct Shell {
    username: String,
    display_name: String,
    internal_id: String,
    token: String,
    cwd: String,
    history: Vec<String>,
}

impl Shell {
    fn new() -> Self {
        let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let display_name = env::var("DISPLAY_NAME").unwrap_or_else(|_| username.clone());
        let internal_id = env::var("AYUX_INTERNAL_ID").unwrap_or_else(|_| "unknown".to_string());
        let token = env::var("AYUX_SESSION_TOKEN").unwrap_or_else(|_| "none".to_string());
        let cwd = if username == "root" {
            paths::ROOT_ROOT.to_string()
        } else {
            paths::user_home(&internal_id)
        };

        Self {
            username,
            display_name,
            internal_id,
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

        libayux::print_heading("Ayux Shell");
        println!("Welcome, {}!", self.display_name);
        println!("Type 'help' for available commands.\n");

        loop {
            let hostname = libayux::get_hostname();
            print!("{}@{}:{}$ ", self.username, hostname, self.cwd);
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
                "confirm_deletion" => self.confirm_deletion(),
                "create_user" => {
                    if parts.len() < 2 {
                        println!("Usage: create_user <username>");
                    } else {
                        self.create_user_wizard(parts[1]);
                    }
                }
                "delete_user" => {
                    if parts.len() < 2 {
                        println!("Usage: delete_user <internal_id>");
                    } else {
                        self.delete_user(parts[1]);
                    }
                }
                "list_users" => self.list_users(),
                "usb_authorize" => self.usb_set_authorized(true),
                "usb_unauthorize" => self.usb_set_authorized(false),
                "usb_status" => self.usb_get_status(),
                cmd => {
                    println!("Unknown command: '{}'", cmd);
                    println!("Type 'help' to see available commands.");
                    self.suggest_command(cmd);
                }
            }
        }
    }

    fn print_help(&self) {
        println!("\nAvailable commands:");
        println!("  help                 - Display this help message");
        println!("  exit, logout         - End the current session");
        println!("  pwd                  - Print current working directory");
        println!("  cd <dir>             - Change current working directory");
        println!("  ls [path]            - List directory contents");
        println!("  cat <file>           - Display file contents");
        println!("  mkdir <dir>          - Create a new directory");
        println!("  touch <file>         - Create a new empty file");
        println!("  echo [text]          - Display a line of text");
        println!("  clear                - Clear the terminal screen");
        println!("  whoami               - Display current user");
        println!("  hostname             - Display system hostname");
        println!("  date                 - Display current system time");
        println!("  reboot               - Restart the system");
        println!("  shutdown             - Shut down the system");
        println!("  history              - Show command history");
        println!("  confirm_deletion     - Confirm deletion of YOUR account if pending");
        println!("  create_user <uname>  - (Admin) Start create user wizard");
        println!("  delete_user <id>     - (Admin) Mark user for deletion");
        println!("  list_users           - (Admin) List display names of accounts");
        println!("  usb_authorize        - (Admin) Authorize USB data sharing");
        println!("  usb_unauthorize      - (Admin) Revoke USB data sharing");
        println!("  usb_status           - (Admin) Get USB authorization status\n");
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
                paths::ROOT_ROOT.to_string()
            } else {
                paths::user_home(&self.internal_id)
            };
            return;
        }
        let new_path = self.resolve_path(path);
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
            AipcMessage::Session(libaipc::SessionRequest::DestroySession {
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

    fn confirm_deletion(&self) {
        println!("YOU ARE ABOUT TO PERMANENTLY DELETE YOUR ACCOUNT AND ALL DATA.");
        println!("This action CANNOT be undone.");
        print!("Enter your password to confirm: ");
        io::stdout().flush().ok();

        let password = self.read_password();

        let mut client = match AipcClient::connect(AUTH_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to auth service: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Auth(AuthRequest::ConfirmDeletion { password }),
        );

        match res {
            Ok(AipcMessage::AuthRes(AuthResponse::Success)) => {
                println!("\nAccount deleted. Logging out...");
                self.logout();
            }
            Ok(AipcMessage::AuthRes(AuthResponse::Error(e))) => println!("\nError: {}", e),
            _ => println!("\nUnexpected response"),
        }
    }

    fn create_user_wizard(&self, username: &str) {
        if let Err(e) = libayux::validate_username(username) {
            println!("Invalid username: {}", e);
            return;
        }

        print!("Enter Display Name: ");
        io::stdout().flush().ok();
        let mut display_name = String::new();
        io::stdin().read_line(&mut display_name).ok();
        let display_name = display_name.trim().to_string();

        print!("Enter Password: ");
        io::stdout().flush().ok();
        let password = self.read_password();
        println!();

        print!("Enter Role (Standard/Administrator): ");
        io::stdout().flush().ok();
        let mut role = String::new();
        io::stdin().read_line(&mut role).ok();
        let role = role.trim().to_string();

        let mut client = match AipcClient::connect(AUTH_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to auth service: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Auth(AuthRequest::CreateUser {
                username: username.to_string(),
                password,
                display_name,
                role,
            }),
        );

        match res {
            Ok(AipcMessage::AuthRes(AuthResponse::Success)) => println!("User created successfully."),
            Ok(AipcMessage::AuthRes(AuthResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn delete_user(&self, internal_id: &str) {
        let mut client = match AipcClient::connect(AUTH_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to auth service: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Auth(AuthRequest::DeleteUser { internal_id: internal_id.to_string() }),
        );

        match res {
            Ok(AipcMessage::AuthRes(AuthResponse::Success)) => println!("User marked for deletion. The user must now log in and confirm deletion."),
            Ok(AipcMessage::AuthRes(AuthResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn list_users(&self) {
        let mut client = match AipcClient::connect(AUTH_SOCKET_PATH) {
            Ok(c) => c,
            Err(e) => {
                println!("Error connecting to auth service: {}", e);
                return;
            }
        };

        let res = client.request(
            "ayux_shell",
            Some(self.token.clone()),
            AipcMessage::Auth(AuthRequest::ListUsers),
        );

        match res {
            Ok(AipcMessage::AuthRes(AuthResponse::UserList(users))) => {
                println!("\nRegistered Users (Display Names):");
                for user in users {
                    println!(" - {}", user);
                }
                println!();
            }
            Ok(AipcMessage::AuthRes(AuthResponse::Error(e))) => println!("Error: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn usb_set_authorized(&self, authorized: bool) {
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
            AipcMessage::Security(SecurityRequest::UsbSetAuthorized { authorized }),
        );

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::Success)) => {
                if authorized {
                    println!("USB data sharing authorized.");
                } else {
                    println!("USB data sharing unauthorized.");
                }
            }
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn usb_get_status(&self) {
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
            AipcMessage::Security(SecurityRequest::UsbGetStatus),
        );

        match res {
            Ok(AipcMessage::SecurityRes(SecurityResponse::UsbStatus { authorized })) => {
                println!("USB Authorization Status: {}", if authorized { "Authorized" } else { "Unauthorized" });
            }
            Ok(AipcMessage::SecurityRes(SecurityResponse::Denied(e))) => println!("Denied: {}", e),
            _ => println!("Unexpected response"),
        }
    }

    fn read_password(&self) -> String {
        use termion::input::TermRead;
        let mut password = String::new();
        if let Ok(Some(p)) = io::stdin().read_passwd(&mut io::stdout()) {
            password = p;
        }
        password
    }

    fn suggest_command(&self, cmd: &str) {
        let commands = [
            "help", "exit", "logout", "pwd", "cd", "ls", "cat", "mkdir", "touch", "echo", "clear",
            "whoami", "hostname", "date", "reboot", "shutdown", "history", "confirm_deletion",
            "create_user", "delete_user", "list_users", "usb_authorize", "usb_unauthorize", "usb_status",
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
