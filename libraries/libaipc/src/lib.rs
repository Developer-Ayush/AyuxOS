use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

pub const AIPC_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageType {
    Request,
    Response,
    Event,
    Error,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AipcHeader {
    pub version: u32,
    pub message_type: MessageType,
    pub sender: String,
    pub session_id: Option<String>,
    pub correlation_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AipcEnvelope {
    pub header: AipcHeader,
    pub message: AipcMessage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthRequest {
    Login {
        username: String,
        password: String,
    },
    ChangePassword {
        username: String,
        old_password: String,
        new_password: String,
    },
    CreateUser {
        username: String,
        password: String,
        display_name: String,
        role: String,
    },
    DeleteUser {
        username: String,
    },
    ListUsers,
    CountUsers,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthResponse {
    Success,
    Authenticated {
        uid: u32,
        username: String,
        role: String,
        capabilities: Vec<String>,
    },
    Error(String),
    UserList(Vec<String>),
    UserCount(usize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SessionRequest {
    CreateSession {
        uid: u32,
        username: String,
        role: String,
        capabilities: Vec<String>,
    },
    DestroySession {
        token: String,
    },
    ValidateSession {
        token: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SessionResponse {
    Success {
        token: String,
    },
    Error(String),
    Valid {
        uid: u32,
        username: String,
        role: String,
        capabilities: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SecurityRequest {
    Authorize { operation: String, path: String },
    FsLs { path: String },
    FsRead { path: String },
    FsWrite { path: String, content: Vec<u8> },
    FsMkdir { path: String },
    FsTouch { path: String },
    PowerReboot,
    PowerShutdown,
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
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LogRequest {
    Log {
        level: LogLevel,
        module: String,
        message: String,
        timestamp: u64,
    },
    Query {
        min_level: Option<LogLevel>,
        module: Option<String>,
        limit: usize,
    },
    GetKernelLogs,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LogResponse {
    Success,
    Logs(Vec<String>),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetworkRequest {
    ListInterfaces,
    GetInterfaceConfig { name: String },
    ConfigureInterface { name: String, up: bool, dhcp: bool },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetworkResponse {
    Interfaces(Vec<String>),
    InterfaceConfig {
        name: String,
        up: bool,
        ip: Option<String>,
        mac: String,
    },
    Success,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HalRequest {
    DisplayGetInfo,
    InputGetDevices,
    StorageGetDevices,
    PowerReboot,
    PowerShutdown,
    ClockGetTime,
    RandomGet { length: usize },
    DiscoveryScan,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HalResponse {
    DisplayInfo { width: u32, height: u32, bpp: u32 },
    InputDevices(Vec<String>),
    StorageDevices(Vec<String>),
    ClockTime(u64),
    RandomData(Vec<u8>),
    DiscoveryResult(Vec<String>),
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
    Log(LogRequest),
    LogRes(LogResponse),
    Network(NetworkRequest),
    NetworkRes(NetworkResponse),
    Hal(HalRequest),
    HalRes(HalResponse),
    GenericError(String),
}

pub struct AipcClient {
    stream: UnixStream,
    timeout: Option<std::time::Duration>,
}

impl AipcClient {
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        Ok(Self {
            stream,
            timeout: None,
        })
    }

    pub fn from_stream(stream: UnixStream) -> Self {
        Self {
            stream,
            timeout: None,
        }
    }

    pub fn set_timeout(&mut self, timeout: Option<std::time::Duration>) -> io::Result<()> {
        self.timeout = timeout;
        self.stream.set_read_timeout(timeout)?;
        self.stream.set_write_timeout(timeout)?;
        Ok(())
    }

    pub fn send_envelope(&mut self, envelope: &AipcEnvelope) -> io::Result<()> {
        let encoded: Vec<u8> = bincode::serialize(envelope).map_err(io::Error::other)?;
        let len = encoded.len() as u32;
        self.stream.write_all(&len.to_le_bytes())?;
        self.stream.write_all(&encoded)?;
        Ok(())
    }

    pub fn receive_envelope(&mut self) -> io::Result<AipcEnvelope> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut buffer = vec![0u8; len];
        self.stream.read_exact(&mut buffer)?;

        let envelope: AipcEnvelope = bincode::deserialize(&buffer).map_err(io::Error::other)?;
        Ok(envelope)
    }

    pub fn request(
        &mut self,
        sender: &str,
        session_id: Option<String>,
        msg: AipcMessage,
    ) -> io::Result<AipcMessage> {
        let correlation_id = rand::random::<u64>();
        let envelope = AipcEnvelope {
            header: AipcHeader {
                version: AIPC_VERSION,
                message_type: MessageType::Request,
                sender: sender.to_string(),
                session_id,
                correlation_id,
            },
            message: msg,
        };

        self.send_envelope(&envelope)?;
        let response_env = self.receive_envelope()?;

        if response_env.header.correlation_id != correlation_id {
            return Err(io::Error::other("Correlation ID mismatch"));
        }

        Ok(response_env.message)
    }
}

pub fn create_listener<P: AsRef<Path>>(path: P) -> io::Result<UnixListener> {
    if path.as_ref().exists() {
        std::fs::remove_file(path.as_ref())?;
    }
    UnixListener::bind(path)
}
