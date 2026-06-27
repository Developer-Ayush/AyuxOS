use libaipc::{AipcMessage, AuthRequest, AuthResponse, create_listener};
use serde::{Serialize, Deserialize};
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::collections::HashMap;

const AUTH_DB_PATH: &str = "/root/auth/users.db";
const AUTH_SOCKET_PATH: &str = "/run/auth.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserRecord {
    username: String,
    uid: u32,
    password_hash: String,
    metadata: HashMap<String, String>,
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

        // Ensure root user exists if db is empty
        if service.users.is_empty() {
            service.create_initial_root();
        }

        service
    }

    fn create_initial_root(&mut self) {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password("ayuxos".as_bytes(), &salt).expect("Failed to hash root password").to_string();

        let root = UserRecord {
            username: "root".to_string(),
            uid: 0,
            password_hash,
            metadata: HashMap::new(),
        };
        self.users.insert("root".to_string(), root);
        self.save_db();
    }

    fn load_db(&mut self) {
        if Path::new(AUTH_DB_PATH).exists() {
            let mut file = File::open(AUTH_DB_PATH).expect("Failed to open auth db");
            let mut content = String::new();
            file.read_to_string(&mut content).expect("Failed to read auth db");
            self.users = serde_json::from_str(&content).unwrap_or_default();
        }
    }

    fn save_db(&self) {
        let parent = Path::new(AUTH_DB_PATH).parent().unwrap();
        fs::create_dir_all(parent).expect("Failed to create auth db directory");

        let content = serde_json::to_string_pretty(&self.users).expect("Failed to serialize auth db");
        let mut file = File::create(AUTH_DB_PATH).expect("Failed to create auth db");
        file.write_all(content.as_bytes()).expect("Failed to write auth db");
    }

    fn validate_root_token(&self, token: &str) -> bool {
        use libaipc::{AipcClient, SessionRequest, SessionResponse};
        let mut client = match AipcClient::connect(SESSION_SOCKET_PATH) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let _ = client.send_message(&AipcMessage::Session(SessionRequest::ValidateSession { token: token.to_string() }));
        match client.receive_message() {
            Ok(AipcMessage::SessionRes(SessionResponse::Valid { username, .. })) => username == "root",
            _ => false,
        }
    }

    fn handle_request(&mut self, request: AuthRequest) -> AuthResponse {
        match request {
            AuthRequest::Login { username, password } => {
                if let Some(user) = self.users.get(&username) {
                    let parsed_hash = PasswordHash::new(&user.password_hash).expect("Invalid password hash in DB");
                    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                        AuthResponse::Authenticated { uid: user.uid, username: user.username.clone() }
                    } else {
                        AuthResponse::Error("Invalid password".to_string())
                    }
                } else {
                    AuthResponse::Error("User not found".to_string())
                }
            },
            AuthRequest::ChangePassword { username, old_password, new_password } => {
                 if let Some(user) = self.users.get_mut(&username) {
                    let parsed_hash = PasswordHash::new(&user.password_hash).expect("Invalid password hash in DB");
                    if Argon2::default().verify_password(old_password.as_bytes(), &parsed_hash).is_ok() {
                        let salt = SaltString::generate(&mut OsRng);
                        user.password_hash = Argon2::default().hash_password(new_password.as_bytes(), &salt)
                            .expect("Failed to hash password").to_string();
                        self.save_db();
                        AuthResponse::Success
                    } else {
                        AuthResponse::Error("Invalid old password".to_string())
                    }
                } else {
                    AuthResponse::Error("User not found".to_string())
                }
            },
            AuthRequest::CreateUser { token, username, password } => {
                if !self.validate_root_token(&token) {
                    return AuthResponse::Error("Permission denied: Root only".to_string());
                }
                if self.users.contains_key(&username) {
                    return AuthResponse::Error("User already exists".to_string());
                }

                let uid = (self.users.values().map(|u| u.uid).max().unwrap_or(1000) + 1).max(1001);
                let salt = SaltString::generate(&mut OsRng);
                let password_hash = Argon2::default().hash_password(password.as_bytes(), &salt)
                    .expect("Failed to hash password").to_string();

                let user = UserRecord {
                    username: username.clone(),
                    uid,
                    password_hash,
                    metadata: HashMap::new(),
                };
                self.users.insert(username, user);
                self.save_db();
                AuthResponse::Success
            },
            AuthRequest::DeleteUser { token, username } => {
                if !self.validate_root_token(&token) {
                    return AuthResponse::Error("Permission denied: Root only".to_string());
                }
                if username == "root" {
                    return AuthResponse::Error("Cannot delete root".to_string());
                }
                if self.users.remove(&username).is_some() {
                    self.save_db();
                    AuthResponse::Success
                } else {
                    AuthResponse::Error("User not found".to_string())
                }
            },
            AuthRequest::ListUsers { token } => {
                if !self.validate_root_token(&token) {
                    return AuthResponse::Error("Permission denied: Root only".to_string());
                }
                let usernames = self.users.keys().cloned().collect();
                AuthResponse::UserList(usernames)
            }
        }
    }
}

fn main() {
    println!("[Auth Service] Starting...");

    let mut service = AuthService::new();
    let listener = create_listener(AUTH_SOCKET_PATH).expect("Failed to create auth socket");

    println!("[Auth Service] Listening on {}", AUTH_SOCKET_PATH);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut client = libaipc::AipcClient::from_stream(stream);
                match client.receive_message() {
                    Ok(AipcMessage::Auth(req)) => {
                        let res = service.handle_request(req);
                        let _ = client.send_message(&AipcMessage::AuthRes(res));
                    },
                    _ => eprintln!("[Auth Service] Received invalid message"),
                }
            },
            Err(e) => eprintln!("[Auth Service] Connection error: {}", e),
        }
    }
}
