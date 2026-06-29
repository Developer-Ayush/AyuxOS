use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use libaipc::LogLevel;
use libaipc::{
    AIPC_VERSION, AipcClient, AipcEnvelope, AipcHeader, AipcMessage, AuthRequest, AuthResponse,
    MessageType, create_listener,
};
use libayux::paths;
use libayux::{ayux_log, generate_random_bytes, hmac_sha256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use uuid::Uuid;

const AUTH_SOCKET_PATH: &str = paths::AUTH_SOCKET;
const SESSION_SOCKET_PATH: &str = paths::SESSION_SOCKET;

// Centralized path constants for Auth Service
fn system_secret_path() -> String {
    format!("{}/system_secret", paths::AYUX_SECURITY)
}

fn auth_db_path() -> String {
    format!("{}/auth/users.db", paths::ROOT_ROOT)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserRecord {
    internal_id: String,
    userid_hash: String, // hex encoded HMAC(system_secret, username)
    display_name: String,
    password_hash: String,
    role: String,
    capabilities: Vec<String>,
    state: String, // "active", "pending_deletion"
}

// For migration only
#[derive(Serialize, Deserialize, Clone, Debug)]
struct LegacyUserRecord {
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
    users: HashMap<String, UserRecord>, // Key is userid_hash
    system_secret: [u8; 32],
}

impl AuthService {
    fn new() -> Self {
        let secret = Self::load_or_generate_secret();
        let mut service = Self {
            users: HashMap::new(),
            system_secret: secret,
        };
        service.load_db();
        service
    }

    fn load_or_generate_secret() -> [u8; 32] {
        let secret_path = system_secret_path();
        if Path::new(&secret_path).exists() {
            let mut file = File::open(&secret_path).expect("Failed to open system secret");
            let mut buf = [0u8; 32];
            file.read_exact(&mut buf).expect("Failed to read system secret");
            buf
        } else {
            let parent = Path::new(&secret_path).parent().unwrap();
            fs::create_dir_all(parent).expect("Failed to create security dir");
            let secret = generate_random_bytes(32);
            let mut file = File::create(&secret_path).expect("Failed to create system secret");
            file.write_all(&secret).expect("Failed to write system secret");
            secret.try_into().unwrap()
        }
    }

    fn load_db(&mut self) {
        let db_path = auth_db_path();
        if Path::new(&db_path).exists() {
            match File::open(&db_path) {
                Ok(mut file) => {
                    let mut content = String::new();
                    if let Ok(_size) = file.read_to_string(&mut content) {
                        // Try parsing new format first
                        match serde_json::from_str::<HashMap<String, UserRecord>>(&content) {
                            Ok(users) => self.users = users,
                            Err(_) => {
                                // Try legacy format
                                if let Ok(legacy_users) = serde_json::from_str::<HashMap<String, LegacyUserRecord>>(&content) {
                                    ayux_log(LogLevel::Info, "auth_service", "Migrating legacy user database...");
                                    self.migrate_db(legacy_users);
                                } else {
                                    ayux_log(LogLevel::Error, "auth_service", "Failed to parse auth db (unknown format)");
                                }
                            }
                        }
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

    fn migrate_db(&mut self, legacy_users: HashMap<String, LegacyUserRecord>) {
        for (username, legacy) in legacy_users {
            let internal_id = Uuid::new_v4().to_string();
            let userid_hash = hex::encode(hmac_sha256(&self.system_secret, username.as_bytes()));

            let new_record = UserRecord {
                internal_id: internal_id.clone(),
                userid_hash: userid_hash.clone(),
                display_name: legacy.display_name,
                password_hash: legacy.password_hash,
                role: legacy.role,
                capabilities: legacy.capabilities,
                state: legacy.state,
            };

            // Migrate home directory
            let old_home = format!("{}/{}", paths::USERS_ROOT, username);
            let new_home = paths::user_home(&internal_id);
            if Path::new(&old_home).exists() {
                if let Err(e) = fs::rename(&old_home, &new_home) {
                    ayux_log(LogLevel::Error, "auth_service", &format!("Failed to migrate home dir for {}: {}", username, e));
                }
            }

            self.users.insert(userid_hash, new_record);
        }
        self.save_db();
        ayux_log(LogLevel::Info, "auth_service", "Migration complete.");
    }

    fn save_db(&self) {
        let db_path = auth_db_path();
        if let Some(parent) = Path::new(&db_path).parent()
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

        match File::create(&db_path) {
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

    fn validate_admin_token(&self, token: &str) -> bool {
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
            Ok(AipcMessage::SessionRes(libaipc::SessionResponse::Valid { role, .. })) => {
                role == "Administrator"
            }
            _ => false,
        }
    }

    fn handle_request(&mut self, request: AuthRequest, header: &AipcHeader) -> AuthResponse {
        match request {
            AuthRequest::Login { username, password } => {
                let userid_hash = hex::encode(hmac_sha256(&self.system_secret, username.as_bytes()));
                if let Some(user) = self.users.get(&userid_hash) {
                    let parsed_hash = match PasswordHash::new(&user.password_hash) {
                        Ok(h) => h,
                        Err(_) => {
                            return AuthResponse::Error("Authentication failed.".to_string());
                        }
                    };
                    if Argon2::default()
                        .verify_password(password.as_bytes(), &parsed_hash)
                        .is_ok()
                    {
                        AuthResponse::Authenticated {
                            internal_id: user.internal_id.clone(),
                            username, // Only in memory, returned to the caller
                            display_name: user.display_name.clone(),
                            role: user.role.clone(),
                            capabilities: user.capabilities.clone(),
                        }
                    } else {
                        AuthResponse::Error("Authentication failed.".to_string())
                    }
                } else {
                    // Constant time-ish delay could be added here
                    AuthResponse::Error("Authentication failed.".to_string())
                }
            }
            AuthRequest::ChangePassword {
                username,
                old_password,
                new_password,
            } => {
                let userid_hash = hex::encode(hmac_sha256(&self.system_secret, username.as_bytes()));
                if let Some(user) = self.users.get_mut(&userid_hash) {
                    let parsed_hash = match PasswordHash::new(&user.password_hash) {
                        Ok(h) => h,
                        Err(_) => {
                            return AuthResponse::Error("Authentication failed.".to_string());
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
                        AuthResponse::Error("Authentication failed.".to_string())
                    }
                } else {
                    AuthResponse::Error("Authentication failed.".to_string())
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
                    if !self.validate_admin_token(token) {
                        return AuthResponse::Error("Permission denied: Administrator only".to_string());
                    }
                }

                if let Err(e) = libayux::validate_username(&username) {
                    return AuthResponse::Error(e);
                }

                let userid_hash = hex::encode(hmac_sha256(&self.system_secret, username.as_bytes()));
                if self.users.contains_key(&userid_hash) {
                    return AuthResponse::Error("User already exists".to_string());
                }

                let salt = SaltString::generate(&mut OsRng);
                let password_hash =
                    match Argon2::default().hash_password(password.as_bytes(), &salt) {
                        Ok(h) => h.to_string(),
                        Err(e) => {
                            return AuthResponse::Error(format!("Failed to hash password: {}", e));
                        }
                    };

                let internal_id = Uuid::new_v4().to_string();
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
                    internal_id: internal_id.clone(),
                    userid_hash: userid_hash.clone(),
                    display_name,
                    password_hash,
                    role,
                    capabilities,
                    state: "active".to_string(),
                };

                let home_dir = paths::user_home(&internal_id);
                if let Err(e) = self.create_home_dir(&home_dir) {
                    return AuthResponse::Error(format!("Failed to create home directory: {}", e));
                }

                ayux_log(
                    LogLevel::Info,
                    "auth_service",
                    "New user account created successfully.",
                );
                self.users.insert(userid_hash, user);
                self.save_db();
                AuthResponse::Success
            }
            AuthRequest::DeleteUser { internal_id } => {
                let token = match &header.session_id {
                    Some(t) => t,
                    None => return AuthResponse::Error("Missing session token".to_string()),
                };
                if !self.validate_admin_token(token) {
                    return AuthResponse::Error("Permission denied: Administrator only".to_string());
                }

                // Find user by internal_id
                let userid_hash = self.users.iter()
                    .find(|(_, u)| u.internal_id == internal_id)
                    .map(|(k, _)| k.clone());

                if let Some(hash) = userid_hash {
                    if let Some(user) = self.users.get_mut(&hash) {
                        user.state = "pending_deletion".to_string();
                        self.save_db();
                        ayux_log(LogLevel::Info, "auth_service", &format!("User {} marked for deletion", internal_id));
                        AuthResponse::Success
                    } else {
                        AuthResponse::Error("User not found".to_string())
                    }
                } else {
                    AuthResponse::Error("User not found".to_string())
                }
            }
            AuthRequest::ConfirmDeletion { password } => {
                let token = match &header.session_id {
                    Some(t) => t,
                    None => return AuthResponse::Error("Missing session token".to_string()),
                };

                // Get user from session
                let mut client = match AipcClient::connect(SESSION_SOCKET_PATH) {
                    Ok(c) => c,
                    Err(_) => return AuthResponse::Error("Session service unavailable".to_string()),
                };

                let res = client.request(
                    "auth_service",
                    None,
                    AipcMessage::Session(libaipc::SessionRequest::ValidateSession {
                        token: token.to_string(),
                    }),
                );

                let internal_id = match res {
                    Ok(AipcMessage::SessionRes(libaipc::SessionResponse::Valid { internal_id, .. })) => internal_id,
                    _ => return AuthResponse::Error("Invalid session".to_string()),
                };

                let userid_hash = self.users.iter()
                    .find(|(_, u)| u.internal_id == internal_id)
                    .map(|(k, _)| k.clone());

                if let Some(hash) = userid_hash {
                    let user = self.users.get(&hash).unwrap();
                    if user.state != "pending_deletion" {
                        return AuthResponse::Error("Deletion not pending for this account".to_string());
                    }

                    let parsed_hash = match PasswordHash::new(&user.password_hash) {
                        Ok(h) => h,
                        Err(_) => return AuthResponse::Error("Authentication failed.".to_string()),
                    };

                    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                        // DELETE DATA
                        let home_dir = paths::user_home(&internal_id);
                        if let Err(e) = fs::remove_dir_all(&home_dir) {
                            ayux_log(LogLevel::Error, "auth_service", &format!("Failed to delete home dir: {}", e));
                        }

                        self.users.remove(&hash);
                        self.save_db();
                        ayux_log(LogLevel::Info, "auth_service", &format!("User {} permanently deleted", internal_id));
                        AuthResponse::Success
                    } else {
                        AuthResponse::Error("Authentication failed.".to_string())
                    }
                } else {
                    AuthResponse::Error("User not found".to_string())
                }
            }
            AuthRequest::ListUsers => {
                let token = match &header.session_id {
                    Some(t) => t,
                    None => return AuthResponse::Error("Missing session token".to_string()),
                };
                if !self.validate_admin_token(token) {
                    return AuthResponse::Error("Permission denied: Administrator only".to_string());
                }
                // Return Display Names instead of UserIDs
                let display_names = self.users.values().map(|u| u.display_name.clone()).collect();
                AuthResponse::UserList(display_names)
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
