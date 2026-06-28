use libaipc::{
    AIPC_VERSION, AipcClient, AipcEnvelope, AipcHeader, AipcMessage, LogLevel, LogRequest,
    LogResponse, MessageType, create_listener,
};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_SOCKET_PATH: &str = "/run/log.sock";
const KMSG_PATH: &str = "/proc/kmsg";
const LOG_DIR: &str = "/var/log";
const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024; // 5 MB
const MAX_ROTATED_FILES: usize = 5;

struct LogService {
    log_files: Arc<Mutex<HashMap<String, fs::File>>>,
}

impl LogService {
    fn new() -> io::Result<Self> {
        fs::create_dir_all(LOG_DIR)?;
        Ok(Self {
            log_files: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn get_log_path(&self, module: &str) -> String {
        let filename = match module.to_lowercase().as_str() {
            "auth_service" => "auth.log",
            "security_manager" => "security.log",
            "network_manager" => "network.log",
            "kernel" => "kernel.log",
            _ => "system.log",
        };
        format!("{}/{}", LOG_DIR, filename)
    }

    fn rotate_if_needed(&self, path: &str) -> io::Result<()> {
        let metadata = fs::metadata(path)?;
        if metadata.len() < MAX_LOG_SIZE {
            return Ok(());
        }

        // Rotate
        for i in (1..MAX_ROTATED_FILES).rev() {
            let old = format!("{}.{}", path, i);
            let new = format!("{}.{}", path, i + 1);
            if Path::new(&old).exists() {
                let _ = fs::rename(old, new);
            }
        }
        let _ = fs::rename(path, format!("{}.1", path));
        Ok(())
    }

    fn write_log(&self, level: LogLevel, module: &str, message: &str, timestamp: u64) {
        let log_line = format!("[{}] [{:?}] [{}] {}\n", timestamp, level, module, message);

        // Debug levels might not be printed to stdout in the future, but for now we keep it
        // print!("{}", log_line);

        let path = self.get_log_path(module);

        if let Ok(mut files) = self.log_files.lock() {
            let file = files.entry(path.clone()).or_insert_with(|| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .expect("Failed to open log file")
            });

            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();

            if let Ok(metadata) = file.metadata()
                && metadata.len() >= MAX_LOG_SIZE
            {
                // Close the file so we can rename it
                files.remove(&path);
                let _ = self.rotate_if_needed(&path);
            }
        }
    }

    fn handle_request(&self, request: LogRequest) -> LogResponse {
        match request {
            LogRequest::Log {
                level,
                module,
                message,
                timestamp,
            } => {
                self.write_log(level, &module, &message, timestamp);
                LogResponse::Success
            }
            LogRequest::Query { .. } => LogResponse::Error("Query not implemented".to_string()),
            LogRequest::GetKernelLogs => {
                LogResponse::Error("GetKernelLogs not implemented via IPC yet".to_string())
            }
        }
    }
}

fn main() -> io::Result<()> {
    let service = match LogService::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("[Log Service] FATAL: Failed to initialize: {}", e);
            return Err(e);
        }
    };

    // Start kernel log collector
    let service_clone = Arc::clone(&service);
    thread::spawn(move || {
        if let Ok(file) = fs::File::open(KMSG_PATH) {
            let reader = BufReader::new(file);
            for l in reader.lines().map_while(Result::ok) {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                service_clone.write_log(LogLevel::Info, "Kernel", &l, ts);
            }
        } else {
            eprintln!("[Log Service] Failed to open {}", KMSG_PATH);
        }
    });

    let listener = create_listener(LOG_SOCKET_PATH)?;
    println!("[Log Service] Listening on {}", LOG_SOCKET_PATH);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let service_clone = Arc::clone(&service);
                thread::spawn(move || {
                    let mut client = AipcClient::from_stream(stream);
                    loop {
                        match client.receive_envelope() {
                            Ok(envelope) => {
                                if let AipcMessage::Log(req) = envelope.message {
                                    let res = service_clone.handle_request(req);
                                    let response_env = AipcEnvelope {
                                        header: AipcHeader {
                                            version: AIPC_VERSION,
                                            message_type: MessageType::Response,
                                            sender: "log_service".to_string(),
                                            session_id: None,
                                            correlation_id: envelope.header.correlation_id,
                                        },
                                        message: AipcMessage::LogRes(res),
                                    };
                                    if let Err(e) = client.send_envelope(&response_env) {
                                        eprintln!("[Log Service] Failed to send response: {}", e);
                                        break;
                                    }
                                }
                            }
                            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                            Err(e) => {
                                eprintln!("[Log Service] IPC error: {}", e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => eprintln!("[Log Service] Connection error: {}", e),
        }
    }
    Ok(())
}
