use libaipc::{
    AIPC_VERSION, AipcClient, AipcEnvelope, AipcHeader, AipcMessage, MessageType, SessionRequest,
    SessionResponse, create_listener,
};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::io;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const SESSION_SOCKET_PATH: &str = "/run/session.sock";

struct Session {
    #[allow(dead_code)]
    token: String,
    uid: u32,
    username: String,
    role: String,
    capabilities: Vec<String>,
    child_pid: Option<u32>,
}

struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    fn create_session(
        &mut self,
        uid: u32,
        username: String,
        role: String,
        capabilities: Vec<String>,
    ) -> SessionResponse {
        let token = Uuid::new_v4().to_string();

        let mut session = Session {
            token: token.clone(),
            uid,
            username: username.clone(),
            role,
            capabilities,
            child_pid: None,
        };

        // Start user shell in background
        match self.launch_user_shell(&token, &username) {
            Ok(pid) => {
                session.child_pid = Some(pid);
                self.sessions.insert(token.clone(), session);
                SessionResponse::Success { token }
            }
            Err(e) => SessionResponse::Error(format!("Failed to launch shell: {}", e)),
        }
    }

    fn launch_user_shell(&self, token: &str, username: &str) -> std::io::Result<u32> {
        use nix::sched::{CloneFlags, unshare};
        use nix::unistd::setsid;
        use std::os::unix::process::CommandExt;

        let username = username.to_string();

        unsafe {
            let child = Command::new("/bin/ayux_shell")
                .env("USER", &username)
                .env("AYUX_SESSION_TOKEN", token)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .pre_exec(move || {
                    // Create a new session and process group
                    setsid().map_err(std::io::Error::other)?;

                    // Isolate namespaces
                    unshare(
                        CloneFlags::CLONE_NEWNS
                            | CloneFlags::CLONE_NEWPID
                            | CloneFlags::CLONE_NEWUTS
                            | CloneFlags::CLONE_NEWIPC,
                    )
                    .map_err(std::io::Error::other)?;

                    Ok(())
                })
                .spawn()?;
            Ok(child.id())
        }
    }

    fn destroy_session(&mut self, token: String) -> SessionResponse {
        if let Some(session) = self.sessions.remove(&token) {
            if let Some(pid) = session.child_pid {
                // Kill the entire process group
                let _ = signal::kill(Pid::from_raw(-(pid as i32)), Signal::SIGTERM);
                thread::sleep(Duration::from_millis(100));
                let _ = signal::kill(Pid::from_raw(-(pid as i32)), Signal::SIGKILL);
            }
            SessionResponse::Success { token }
        } else {
            SessionResponse::Error("Session not found".to_string())
        }
    }

    fn validate_session(&self, token: String) -> SessionResponse {
        if let Some(session) = self.sessions.get(&token) {
            SessionResponse::Valid {
                uid: session.uid,
                username: session.username.clone(),
                role: session.role.clone(),
                capabilities: session.capabilities.clone(),
            }
        } else {
            SessionResponse::Error("Invalid session".to_string())
        }
    }

    fn handle_request(&mut self, request: SessionRequest) -> SessionResponse {
        match request {
            SessionRequest::CreateSession {
                uid,
                username,
                role,
                capabilities,
            } => self.create_session(uid, username, role, capabilities),
            SessionRequest::DestroySession { token } => self.destroy_session(token),
            SessionRequest::ValidateSession { token } => self.validate_session(token),
        }
    }
}

fn main() -> io::Result<()> {
    let mut manager = SessionManager::new();
    let listener = create_listener(SESSION_SOCKET_PATH)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut client = AipcClient::from_stream(stream);
                loop {
                    match client.receive_envelope() {
                        Ok(envelope) => {
                            if let AipcMessage::Session(req) = envelope.message {
                                let res = manager.handle_request(req);
                                let response_env = AipcEnvelope {
                                    header: AipcHeader {
                                        version: AIPC_VERSION,
                                        message_type: MessageType::Response,
                                        sender: "session_manager".to_string(),
                                        session_id: None,
                                        correlation_id: envelope.header.correlation_id,
                                    },
                                    message: AipcMessage::SessionRes(res),
                                };
                                let _ = client.send_envelope(&response_env);
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                        Err(_) => break,
                    }
                }
            }
            Err(e) => eprintln!("[Session Manager] Connection error: {}", e),
        }
    }
    Ok(())
}
