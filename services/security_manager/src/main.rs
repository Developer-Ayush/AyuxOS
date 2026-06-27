use libaipc::{AipcMessage, SecurityRequest, SecurityResponse, SessionRequest, SessionResponse, create_listener, AipcClient};
use std::fs::{self, File};
use std::path::PathBuf;

const SECURITY_SOCKET_PATH: &str = "/run/security.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

struct SecurityManager {
    // In a real implementation, we would have a more complex policy engine
}

impl SecurityManager {
    fn new() -> Self {
        Self {}
    }

    fn validate_session(&self, token: &str) -> Option<(u32, String)> {
        // Connect to Session Manager to validate token
        let mut client = AipcClient::connect(SESSION_SOCKET_PATH).ok()?;
        client.send_message(&AipcMessage::Session(SessionRequest::ValidateSession { token: token.to_string() })).ok()?;

        match client.receive_message() {
            Ok(AipcMessage::SessionRes(SessionResponse::Valid { uid, username })) => Some((uid, username)),
            _ => None,
        }
    }

    fn is_allowed(&self, _uid: u32, username: &str, _operation: &str, path: &str) -> bool {
        let _path_buf = PathBuf::from(path);

        // Root account restrictions
        if username == "root" {
            // Root can access /root/ and /main/
            if path.starts_with("/root") || path.starts_with("/main") || path.starts_with("/bin") || path.starts_with("/usr") || path.starts_with("/etc") || path.starts_with("/run") || path.starts_with("/tmp") || path.starts_with("/proc") || path.starts_with("/sys") || path.starts_with("/dev") {
                 // But Root cannot access /users/<other_user>
                 if path.starts_with("/users/") {
                     return false;
                 }
                 return true;
            }
            return false;
        }

        // Normal user restrictions
        let user_home = format!("/users/{}", username);
        if path.starts_with(&user_home) || path.starts_with("/tmp") || path.starts_with("/bin") || path.starts_with("/usr") || path.starts_with("/etc") || path.starts_with("/run") || path.starts_with("/proc") || path.starts_with("/sys") || path.starts_with("/dev") {
            // Cannot escape to parent
            if path.contains("..") {
                return false;
            }

            // Cannot access /main, /root, or other users
            if path.starts_with("/main") || path.starts_with("/root") {
                return false;
            }

            if path.starts_with("/users/") && !path.starts_with(&user_home) {
                return false;
            }

            return true;
        }

        false
    }

    fn handle_request(&self, request: SecurityRequest) -> SecurityResponse {
        let (token, operation, path) = match &request {
            SecurityRequest::Authorize { token, operation, path } => (token, operation.as_str(), path.as_str()),
            SecurityRequest::FsLs { token, path } => (token, "ls", path.as_str()),
            SecurityRequest::FsRead { token, path } => (token, "read", path.as_str()),
            SecurityRequest::FsWrite { token, path, .. } => (token, "write", path.as_str()),
            SecurityRequest::FsMkdir { token, path } => (token, "mkdir", path.as_str()),
            SecurityRequest::FsTouch { token, path } => (token, "touch", path.as_str()),
        };

        let (uid, username) = match self.validate_session(token) {
            Some(data) => data,
            None => return SecurityResponse::Denied("Invalid session".to_string()),
        };

        if !self.is_allowed(uid, &username, operation, path) {
            return SecurityResponse::Denied(format!("Permission denied for {} on {}", operation, path));
        }

        match request {
            SecurityRequest::Authorize { .. } => SecurityResponse::Allowed,
            SecurityRequest::FsLs { path, .. } => {
                match fs::read_dir(path) {
                    Ok(entries) => {
                        let names = entries.filter_map(|e| e.ok().map(|entry| entry.file_name().to_string_lossy().into_owned())).collect();
                        SecurityResponse::FsEntries(names)
                    },
                    Err(e) => SecurityResponse::Error(e.to_string()),
                }
            },
            SecurityRequest::FsRead { path, .. } => {
                match fs::read(path) {
                    Ok(content) => SecurityResponse::FsContent(content),
                    Err(e) => SecurityResponse::Error(e.to_string()),
                }
            },
            SecurityRequest::FsWrite { path, content, .. } => {
                match fs::write(path, content) {
                    Ok(_) => SecurityResponse::Success,
                    Err(e) => SecurityResponse::Error(e.to_string()),
                }
            },
            SecurityRequest::FsMkdir { path, .. } => {
                match fs::create_dir_all(path) {
                    Ok(_) => SecurityResponse::Success,
                    Err(e) => SecurityResponse::Error(e.to_string()),
                }
            },
            SecurityRequest::FsTouch { path, .. } => {
                match File::create(path) {
                    Ok(_) => SecurityResponse::Success,
                    Err(e) => SecurityResponse::Error(e.to_string()),
                }
            },
        }
    }
}

fn main() {
    println!("[Security Manager] Starting...");

    let manager = SecurityManager::new();
    let listener = create_listener(SECURITY_SOCKET_PATH).expect("Failed to create security socket");

    println!("[Security Manager] Listening on {}", SECURITY_SOCKET_PATH);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut client = AipcClient::from_stream(stream);
                match client.receive_message() {
                    Ok(AipcMessage::Security(req)) => {
                        let res = manager.handle_request(req);
                        let _ = client.send_message(&AipcMessage::SecurityRes(res));
                    },
                    _ => eprintln!("[Security Manager] Received invalid message"),
                }
            },
            Err(e) => eprintln!("[Security Manager] Connection error: {}", e),
        }
    }
}
