use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use libaipc::LogLevel;
use libaipc::{
    AIPC_VERSION, AipcClient, AipcEnvelope, AipcHeader, AipcMessage, AuthRequest, AuthResponse,
    MessageType, create_listener,
};
use libayux::ayux_log;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

const AUTH_DB_PATH: &str = "/root/auth/users.db";
const AUTH_SOCKET_PATH: &str = "/run/auth.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserRecord {
    username: String,
    display_name: String,
    uid: u32,
    password_hash: String,
    home_dir: String,
    shell: String,
    role: String,
    capabilities: Vec<String>,
    state: String,
}

struct AuthService {
    users: HashMap<String, UserRecord>,
}

impl AuthService {
    fn new() -> Self {
        let mut service = Self {
            users: HashMap::new(),
        };
        service.load_db();
        service
    }

    fn load_db(&mut self) {
        if Path::new(AUTH_DB_PATH).exists() {
            match File::open(AUTH_DB_PATH) {
                Ok(mut file) => {
                    let mut content = String::new();
                    if let Ok(_size) = file.read_to_string(&mut content) {
                        self.users = serde_json::from_str(&content).unwrap_or_default();
                    } else {
                        ayux_log(LogLevel::Error, "auth_service", "Failed to read auth db");
                    }
                }
                Err(e) => {
                    ayux_log(
                        LogLevel::Error,
                        "auth_service",
                        &format!("Failed to open auth db: {}", e),
                    );
                }
            }
        }
    }

    fn save_db(&self) {
        if let Some(parent) = Path::new(AUTH_DB_PATH).parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            ayux_log(
                LogLevel::Error,
                "auth_service",
                &format!("Failed to create auth db directory: {}", e),
            );
            return;
        }

        let content = match serde_json::to_string_pretty(&self.users) {
            Ok(c) => c,
            Err(e) => {
                ayux_log(
                    LogLevel::Error,
                    "auth_service",
                    &format!("Failed to serialize auth db: {}", e),
                );
                return;
            }
        };

        match File::create(AUTH_DB_PATH) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(content.as_bytes()) {
                    ayux_log(
                        LogLevel::Error,
                        "auth_service",
                        &format!("Failed to write auth db: {}", e),
                    );
                }
            }
            Err(e) => {
                ayux_log(
                    LogLevel::Error,
                    "auth_service",
                    &format!("Failed to create auth db: {}", e),
                );
            }
        }
    }

    fn validate_root_token(&self, token: &str) -> bool {
        use libaipc::SessionRequest;
        let mut client = match AipcClient::connect(SESSION_SOCKET_PATH) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let res = client.request(
            "auth_service",
            None,
            AipcMessage::Session(SessionRequest::ValidateSession {
                token: token.to_string(),
            }),
        );
        match res {
            Ok(AipcMessage::SessionRes(libaipc::SessionResponse::Valid { username, .. })) => {
                username == "root"
            }
            _ => false,
        }
    }

    fn handle_request(&mut self, request: AuthRequest, header: &AipcHeader) -> AuthResponse {
        match request {
            AuthRequest::Login { username, password } => {
                if let Some(user) = self.users.get(&username) {
                    if user.state != "active" {
                        return AuthResponse::Error("Account is not active".to_string());
                    }
                    let parsed_hash = match PasswordHash::new(&user.password_hash) {
                        Ok(h) => h,
                        Err(_) => {
                            return AuthResponse::Error("Invalid password hash in DB".to_string());
                        }
                    };
                    if Argon2::default()
                        .verify_password(password.as_bytes(), &parsed_hash)
                        .is_ok()
                    {
                        AuthResponse::Authenticated {
                            uid: user.uid,
                            username: user.username.clone(),
                            role: user.role.clone(),
                            capabilities: user.capabilities.clone(),
                        }
                    } else {
                        AuthResponse::Error("Invalid password.".to_string())
                    }
                } else {
                    AuthResponse::Error("Unknown username.".to_string())
                }
            }
            AuthRequest::ChangePassword {
                username,
                old_password,
                new_password,
            } => {
                if let Some(user) = self.users.get_mut(&username) {
                    let parsed_hash = match PasswordHash::new(&user.password_hash) {
                        Ok(h) => h,
                        Err(_) => {
                            return AuthResponse::Error("Invalid password hash in DB".to_string());
                        }
                    };
                    if Argon2::default()
                        .verify_password(old_password.as_bytes(), &parsed_hash)
                        .is_ok()
                    {
                        let salt = SaltString::generate(&mut OsRng);
                        user.password_hash = Argon2::default()
                            .hash_password(new_password.as_bytes(), &salt)
                            .map(|h| h.to_string())
                            .unwrap_or_else(|_| user.password_hash.clone());
                        self.save_db();
                        AuthResponse::Success
                    } else {
                        AuthResponse::Error("Invalid old password".to_string())
                    }
                } else {
                    AuthResponse::Error("User not found".to_string())
                }
            }
            AuthRequest::CreateUser {
                username,
                password,
                display_name,
                role,
            } => {
                // Allow creating the first user (Administrator) without a session token
                if !self.users.is_empty() {
                    let token = match &header.session_id {
                        Some(t) => t,
                        None => return AuthResponse::Error("Missing session token".to_string()),
                    };
                    if !self.validate_root_token(token) {
                        return AuthResponse::Error("Permission denied: Root only".to_string());
                    }
                }

                if let Err(e) = libayux::validate_username(&username) {
                    return AuthResponse::Error(e);
                }

                if self.users.contains_key(&username) {
                    return AuthResponse::Error("User already exists".to_string());
                }

                let uid = if self.users.is_empty() {
                    1000
                } else {
                    (self.users.values().map(|u| u.uid).max().unwrap_or(1000) + 1).max(1001)
                };

                let salt = SaltString::generate(&mut OsRng);
                let password_hash =
                    match Argon2::default().hash_password(password.as_bytes(), &salt) {
                        Ok(h) => h.to_string(),
                        Err(e) => {
                            return AuthResponse::Error(format!("Failed to hash password: {}", e));
                        }
                    };

                let home_dir = format!("/users/{}", username);
                let capabilities = if role == "Administrator" {
                    vec![
                        "FsRead".to_string(),
                        "FsWrite".to_string(),
                        "NetworkManage".to_string(),
                        "ServiceManage".to_string(),
                        "Admin".to_string(),
                        "DeviceAccess".to_string(),
                    ]
                } else {
                    vec!["FsRead".to_string(), "FsWrite".to_string()]
                };

                let user = UserRecord {
                    username: username.clone(),
                    display_name,
                    uid,
                    password_hash,
                    home_dir: home_dir.clone(),
                    shell: "/bin/ayux_shell".to_string(),
                    role,
                    capabilities,
                    state: "active".to_string(),
                };

                // Create home directory and default subdirectories
                if let Err(e) = self.create_home_dir(&home_dir) {
                    return AuthResponse::Error(format!("Failed to create home directory: {}", e));
                }

                ayux_log(
                    LogLevel::Info,
                    "auth_service",
                    &format!("User created: {}", username),
                );
                self.users.insert(username, user);
                self.save_db();
                AuthResponse::Success
            }
            AuthRequest::DeleteUser { username } => {
                let token = match &header.session_id {
                    Some(t) => t,
                    None => return AuthResponse::Error("Missing session token".to_string()),
                };
                if !self.validate_root_token(token) {
                    return AuthResponse::Error("Permission denied: Root only".to_string());
                }
                if self.users.remove(&username).is_some() {
                    self.save_db();
                    AuthResponse::Success
                } else {
                    AuthResponse::Error("User not found".to_string())
                }
            }
            AuthRequest::ListUsers => {
                let token = match &header.session_id {
                    Some(t) => t,
                    None => return AuthResponse::Error("Missing session token".to_string()),
                };
                if !self.validate_root_token(token) {
                    return AuthResponse::Error("Permission denied: Root only".to_string());
                }
                let usernames = self.users.keys().cloned().collect();
                AuthResponse::UserList(usernames)
            }
            AuthRequest::CountUsers => AuthResponse::UserCount(self.users.len()),
        }
    }

    fn create_home_dir(&self, path: &str) -> io::Result<()> {
        fs::create_dir_all(path)?;
        let subdirs = [
            "Desktop",
            "Documents",
            "Downloads",
            "Pictures",
            "Music",
            "Videos",
            "Config",
        ];
        for subdir in &subdirs {
            fs::create_dir_all(format!("{}/{}", path, subdir))?;
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut service = AuthService::new();
    let listener = create_listener(AUTH_SOCKET_PATH)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut client = AipcClient::from_stream(stream);
                loop {
                    match client.receive_envelope() {
                        Ok(envelope) => {
                            if let AipcMessage::Auth(req) = envelope.message {
                                let res = service.handle_request(req, &envelope.header);
                                let response_env = AipcEnvelope {
                                    header: AipcHeader {
                                        version: AIPC_VERSION,
                                        message_type: MessageType::Response,
                                        sender: "auth_service".to_string(),
                                        session_id: None,
                                        correlation_id: envelope.header.correlation_id,
                                    },
                                    message: AipcMessage::AuthRes(res),
                                };
                                let _ = client.send_envelope(&response_env);
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                        Err(_) => break,
                    }
                }
            }
            Err(e) => eprintln!("[Auth Service] Connection error: {}", e),
        }
    }
    Ok(())
}
