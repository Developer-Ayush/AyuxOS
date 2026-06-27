use std::io::{self, Read, Write};
use std::os::unix::net::{UnixStream, UnixListener};
use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthRequest {
    Login { username: String, password: String },
    ChangePassword { username: String, old_password: String, new_password: String },
    CreateUser { token: String, username: String, password: String },
    DeleteUser { token: String, username: String },
    ListUsers { token: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthResponse {
    Success,
    Authenticated { uid: u32, username: String },
    Error(String),
    UserList(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SessionRequest {
    CreateSession { uid: u32, username: String },
    DestroySession { token: String },
    ValidateSession { token: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SessionResponse {
    Success { token: String },
    Error(String),
    Valid { uid: u32, username: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SecurityRequest {
    Authorize { token: String, operation: String, path: String },
    FsLs { token: String, path: String },
    FsRead { token: String, path: String },
    FsWrite { token: String, path: String, content: Vec<u8> },
    FsMkdir { token: String, path: String },
    FsTouch { token: String, path: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SecurityResponse {
    Allowed,
    Denied(String),
    FsEntries(Vec<String>),
    FsContent(Vec<u8>),
    Success,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AipcMessage {
    Auth(AuthRequest),
    AuthRes(AuthResponse),
    Session(SessionRequest),
    SessionRes(SessionResponse),
    Security(SecurityRequest),
    SecurityRes(SecurityResponse),
}

pub struct AipcClient {
    stream: UnixStream,
}

impl AipcClient {
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        Ok(Self { stream })
    }

    pub fn from_stream(stream: UnixStream) -> Self {
        Self { stream }
    }

    pub fn send_message(&mut self, msg: &AipcMessage) -> io::Result<()> {
        let encoded: Vec<u8> = bincode::serialize(msg).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let len = encoded.len() as u32;
        self.stream.write_all(&len.to_le_bytes())?;
        self.stream.write_all(&encoded)?;
        Ok(())
    }

    pub fn receive_message(&mut self) -> io::Result<AipcMessage> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut buffer = vec![0u8; len];
        self.stream.read_exact(&mut buffer)?;

        let msg: AipcMessage = bincode::deserialize(&buffer).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(msg)
    }
}

pub fn create_listener<P: AsRef<Path>>(path: P) -> io::Result<UnixListener> {
    if path.as_ref().exists() {
        std::fs::remove_file(path.as_ref())?;
    }
    UnixListener::bind(path)
}
