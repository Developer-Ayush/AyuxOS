use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use libaipc::{
    AIPC_VERSION, AipcClient, AipcEnvelope, AipcHeader, AipcMessage, MessageType, SecurityRequest,
    SecurityResponse, SessionRequest, SessionResponse, create_listener,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io;

const SECURITY_SOCKET_PATH: &str = "/run/security.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    FsRead,
    FsWrite,
    NetworkManage,
    ServiceManage,
    Admin,
    DeviceAccess,
}

struct SecurityManager;

impl SecurityManager {
    fn new() -> Self {
        Self
    }

    fn validate_session(&self, token: &str) -> Option<(u32, String, Vec<String>)> {
        let mut client = AipcClient::connect(SESSION_SOCKET_PATH).ok()?;
        let res = client
            .request(
                "security_manager",
                None,
                AipcMessage::Session(SessionRequest::ValidateSession {
                    token: token.to_string(),
                }),
            )
            .ok()?;

        match res {
            AipcMessage::SessionRes(SessionResponse::Valid {
                uid,
                username,
                capabilities,
                ..
            }) => Some((uid, username, capabilities)),
            _ => None,
        }
    }

    fn has_capability(&self, capabilities: &[String], cap: &str) -> bool {
        capabilities.iter().any(|c| c == cap)
    }

    fn is_allowed(
        &self,
        _uid: u32,
        username: &str,
        capabilities: &[String],
        operation: &str,
        path: &str,
    ) -> bool {
        // Enforce capabilities
        match operation {
            "read" | "ls" => {
                if !self.has_capability(capabilities, "FsRead") {
                    return false;
                }
            }
            "write" | "mkdir" | "touch" => {
                if !self.has_capability(capabilities, "FsWrite") {
                    return false;
                }
            }
            "reboot" | "shutdown" => {
                if !self.has_capability(capabilities, "Admin") {
                    return false;
                }
            }
            _ => {}
        }

        // Standard path-based restrictions
        let user_home = format!("/users/{}", username);
        if path.starts_with(&user_home)
            || path.starts_with("/tmp")
            || path.starts_with("/bin")
            || path.starts_with("/usr")
            || path.starts_with("/etc")
            || path.starts_with("/run")
            || path.starts_with("/proc")
            || path.starts_with("/sys")
            || path.starts_with("/dev")
        {
            if path.contains("..") {
                return false;
            }
            if path.starts_with("/main") || path.starts_with("/root") {
                return self.has_capability(capabilities, "Admin");
            }
            if path.starts_with("/users/") && !path.starts_with(&user_home) {
                return self.has_capability(capabilities, "Admin");
            }
            return true;
        }

        self.has_capability(capabilities, "Admin")
    }

    #[allow(dead_code)]
    fn verify_signature(
        &self,
        public_key_bytes: &[u8],
        message: &[u8],
        signature_bytes: &[u8],
    ) -> bool {
        let pk_bytes: &[u8; 32] = match public_key_bytes.try_into() {
            Ok(b) => b,
            Err(_) => return false,
        };
        let verifying_key = match VerifyingKey::from_bytes(pk_bytes) {
            Ok(key) => key,
            Err(_) => return false,
        };
        let sig_bytes: &[u8; 64] = match signature_bytes.try_into() {
            Ok(b) => b,
            Err(_) => return false,
        };
        let signature = Signature::from_bytes(sig_bytes);
        verifying_key.verify(message, &signature).is_ok()
    }

    fn handle_request(&self, header: &AipcHeader, request: SecurityRequest) -> SecurityResponse {
        let (operation, path) = match &request {
            SecurityRequest::Authorize {
                operation, path, ..
            } => (operation.as_str(), path.as_str()),
            SecurityRequest::FsLs { path, .. } => ("ls", path.as_str()),
            SecurityRequest::FsRead { path, .. } => ("read", path.as_str()),
            SecurityRequest::FsWrite { path, .. } => ("write", path.as_str()),
            SecurityRequest::FsMkdir { path, .. } => ("mkdir", path.as_str()),
            SecurityRequest::FsTouch { path, .. } => ("touch", path.as_str()),
            SecurityRequest::PowerReboot => ("reboot", "/"),
            SecurityRequest::PowerShutdown => ("shutdown", "/"),
        };

        let token = match &header.session_id {
            Some(t) => t,
            None => return SecurityResponse::Denied("Missing session token".to_string()),
        };

        let (uid, username, capabilities) = match self.validate_session(token) {
            Some(data) => data,
            None => return SecurityResponse::Denied("Invalid session".to_string()),
        };

        if !self.is_allowed(uid, &username, &capabilities, operation, path) {
            return SecurityResponse::Denied(format!(
                "Permission denied for {} on {}",
                operation, path
            ));
        }

        match request {
            SecurityRequest::Authorize { .. } => SecurityResponse::Allowed,
            SecurityRequest::PowerReboot => {
                use libayux_hal::power::{LinuxPower, Power};
                let power = LinuxPower;
                let _ = power.reboot();
                SecurityResponse::Success
            }
            SecurityRequest::PowerShutdown => {
                use libayux_hal::power::{LinuxPower, Power};
                let power = LinuxPower;
                let _ = power.shutdown();
                SecurityResponse::Success
            }
            SecurityRequest::FsLs { path, .. } => match fs::read_dir(path) {
                Ok(entries) => {
                    let names = entries
                        .filter_map(|e| {
                            e.ok()
                                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                        })
                        .collect();
                    SecurityResponse::FsEntries(names)
                }
                Err(e) => SecurityResponse::Error(e.to_string()),
            },
            SecurityRequest::FsRead { path, .. } => match fs::read(path) {
                Ok(content) => SecurityResponse::FsContent(content),
                Err(e) => SecurityResponse::Error(e.to_string()),
            },
            SecurityRequest::FsWrite { path, content, .. } => match fs::write(path, content) {
                Ok(_) => SecurityResponse::Success,
                Err(e) => SecurityResponse::Error(e.to_string()),
            },
            SecurityRequest::FsMkdir { path, .. } => match fs::create_dir_all(path) {
                Ok(_) => SecurityResponse::Success,
                Err(e) => SecurityResponse::Error(e.to_string()),
            },
            SecurityRequest::FsTouch { path, .. } => match File::create(path) {
                Ok(_) => SecurityResponse::Success,
                Err(e) => SecurityResponse::Error(e.to_string()),
            },
        }
    }
}

fn main() -> io::Result<()> {
    let manager = SecurityManager::new();
    let listener = create_listener(SECURITY_SOCKET_PATH)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut client = AipcClient::from_stream(stream);
                loop {
                    match client.receive_envelope() {
                        Ok(envelope) => {
                            if let AipcMessage::Security(req) = envelope.message {
                                let res = manager.handle_request(&envelope.header, req);
                                let response_env = AipcEnvelope {
                                    header: AipcHeader {
                                        version: AIPC_VERSION,
                                        message_type: MessageType::Response,
                                        sender: "security_manager".to_string(),
                                        session_id: None,
                                        correlation_id: envelope.header.correlation_id,
                                    },
                                    message: AipcMessage::SecurityRes(res),
                                };
                                let _ = client.send_envelope(&response_env);
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                        Err(_) => break,
                    }
                }
            }
            Err(e) => eprintln!("[Security Manager] Connection error: {}", e),
        }
    }
    Ok(())
}
