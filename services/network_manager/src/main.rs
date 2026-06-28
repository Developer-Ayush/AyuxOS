use libaipc::{
    AIPC_VERSION, AipcClient, AipcEnvelope, AipcHeader, AipcMessage, HalRequest, HalResponse,
    MessageType, NetworkRequest, NetworkResponse, create_listener,
};
use libayux_hal::discovery::{Discovery, LinuxDiscovery};
use std::fs;
use std::io::{self};
use std::process::Command;

const NETWORK_SOCKET_PATH: &str = "/run/network.sock";

struct NetworkManager {}

impl NetworkManager {
    fn new() -> Self {
        Self {}
    }

    fn list_interfaces(&self) -> Vec<String> {
        let mut interfaces = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                interfaces.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        interfaces
    }

    fn get_interface_config(&self, name: &str) -> NetworkResponse {
        let base_path = format!("/sys/class/net/{}", name);
        if fs::metadata(&base_path).is_err() {
            return NetworkResponse::Error(format!("Interface {} not found", name));
        }

        let operstate = fs::read_to_string(format!("{}/operstate", base_path)).unwrap_or_default();
        let up = operstate.trim() == "up" || operstate.trim() == "unknown"; // loopback often shows unknown
        let mac = fs::read_to_string(format!("{}/address", base_path))
            .unwrap_or_default()
            .trim()
            .to_string();

        NetworkResponse::InterfaceConfig {
            name: name.to_string(),
            up,
            ip: None, // IP discovery would require ioctl or parsing /proc/net/fib_trie
            mac,
        }
    }

    fn configure_interface(&self, name: &str, up: bool, dhcp: bool) -> NetworkResponse {
        let status = if up { "up" } else { "down" };
        let output = Command::new("ip")
            .args(["link", "set", name, status])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                if up && dhcp {
                    // Try to start a dhcp client if available
                    let _ = Command::new("udhcpc").args(["-i", name, "-n"]).spawn();
                }
                NetworkResponse::Success
            }
            Ok(out) => NetworkResponse::Error(String::from_utf8_lossy(&out.stderr).to_string()),
            Err(e) => NetworkResponse::Error(e.to_string()),
        }
    }

    fn handle_request(&self, request: NetworkRequest) -> NetworkResponse {
        match request {
            NetworkRequest::ListInterfaces => NetworkResponse::Interfaces(self.list_interfaces()),
            NetworkRequest::GetInterfaceConfig { name } => self.get_interface_config(&name),
            NetworkRequest::ConfigureInterface { name, up, dhcp } => {
                self.configure_interface(&name, up, dhcp)
            }
        }
    }

    fn handle_hal_request(&self, request: HalRequest) -> HalResponse {
        match request {
            HalRequest::DiscoveryScan => {
                let discovery = LinuxDiscovery;
                match discovery.scan() {
                    Ok(devices) => HalResponse::DiscoveryResult(
                        devices
                            .into_iter()
                            .map(|d| format!("{} [{}] at {}", d.name, d.class, d.path))
                            .collect(),
                    ),
                    Err(e) => HalResponse::Error(e.to_string()),
                }
            }
            _ => HalResponse::Error("Not implemented in Network Manager".to_string()),
        }
    }
}

fn main() -> io::Result<()> {
    let manager = NetworkManager::new();

    // Initial loopback configuration
    let _ = Command::new("ip")
        .args(["link", "set", "lo", "up"])
        .status();

    let listener = create_listener(NETWORK_SOCKET_PATH)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut client = AipcClient::from_stream(stream);
                loop {
                    match client.receive_envelope() {
                        Ok(envelope) => match envelope.message {
                            AipcMessage::Network(req) => {
                                let res = manager.handle_request(req);
                                let response_env = AipcEnvelope {
                                    header: AipcHeader {
                                        version: AIPC_VERSION,
                                        message_type: MessageType::Response,
                                        sender: "network_manager".to_string(),
                                        session_id: None,
                                        correlation_id: envelope.header.correlation_id,
                                    },
                                    message: AipcMessage::NetworkRes(res),
                                };
                                let _ = client.send_envelope(&response_env);
                            }
                            AipcMessage::Hal(req) => {
                                let res = manager.handle_hal_request(req);
                                let response_env = AipcEnvelope {
                                    header: AipcHeader {
                                        version: AIPC_VERSION,
                                        message_type: MessageType::Response,
                                        sender: "network_manager".to_string(),
                                        session_id: None,
                                        correlation_id: envelope.header.correlation_id,
                                    },
                                    message: AipcMessage::HalRes(res),
                                };
                                let _ = client.send_envelope(&response_env);
                            }
                            _ => {}
                        },
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                        Err(_) => break,
                    }
                }
            }
            Err(e) => eprintln!("[Network Manager] Connection error: {}", e),
        }
    }
    Ok(())
}
