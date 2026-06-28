use libaipc::{AipcMessage, LogRequest, LogResponse, LogLevel, AipcClient, AipcHeader, AipcEnvelope, MessageType, AIPC_VERSION, create_listener};
use std::fs::{self, OpenOptions};
use std::io::{self, Write, BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_SOCKET_PATH: &str = "/run/log.sock";
const KMSG_PATH: &str = "/proc/kmsg";
const LOG_FILE_PATH: &str = "/var/log/syslog";

struct LogService {
    log_file: Arc<Mutex<fs::File>>,
}

impl LogService {
    fn new() -> Self {
        fs::create_dir_all("/var/log").expect("Failed to create log directory");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE_PATH)
            .expect("Failed to open syslog file");

        Self {
            log_file: Arc::new(Mutex::new(file)),
        }
    }

    fn write_log(&self, level: LogLevel, module: &str, message: &str, timestamp: u64) {
        let log_line = format!("[{}] [{:?}] [{}] {}\n", timestamp, level, module, message);
        print!("{}", log_line);
        let mut file = self.log_file.lock().unwrap();
        let _ = file.write_all(log_line.as_bytes());
    }

    fn handle_request(&self, request: LogRequest) -> LogResponse {
        match request {
            LogRequest::Log { level, module, message, timestamp } => {
                self.write_log(level, &module, &message, timestamp);
                LogResponse::Success
            },
            LogRequest::Query { .. } => {
                LogResponse::Error("Query not implemented".to_string())
            },
            LogRequest::GetKernelLogs => {
                LogResponse::Error("GetKernelLogs not implemented via IPC yet".to_string())
            }
        }
    }
}

fn main() {
    println!("[Log Service] Starting...");

    let service = Arc::new(LogService::new());

    // Start kernel log collector
    let service_clone = Arc::clone(&service);
    thread::spawn(move || {
        if let Ok(file) = fs::File::open(KMSG_PATH) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    service_clone.write_log(LogLevel::Info, "Kernel", &l, ts);
                }
            }
        } else {
            eprintln!("[Log Service] Failed to open {}", KMSG_PATH);
        }
    });

    let listener = create_listener(LOG_SOCKET_PATH).expect("Failed to create log socket");
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
                            },
                            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                            Err(e) => {
                                eprintln!("[Log Service] IPC error: {}", e);
                                break;
                            }
                        }
                    }
                });
            },
            Err(e) => eprintln!("[Log Service] Connection error: {}", e),
        }
    }
}
