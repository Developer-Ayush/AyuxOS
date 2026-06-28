use libaipc::{AipcClient, AipcMessage, AuthRequest, AuthResponse, SessionRequest, SessionResponse, AipcEnvelope, AipcHeader, MessageType, AIPC_VERSION};
use std::io::{self, Write};

const AUTH_SOCKET_PATH: &str = "/run/auth.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

fn main() {
    println!("--- AyuxOS Login ---");

    loop {
        print!("Username: ");
        io::stdout().flush().unwrap();
        let mut username = String::new();
        io::stdin().read_line(&mut username).unwrap();
        let username = username.trim();

        if username.is_empty() { continue; }

        print!("Password: ");
        io::stdout().flush().unwrap();
        let mut password = String::new();
        // In a real OS, we would disable echo here
        io::stdin().read_line(&mut password).unwrap();
        let password = password.trim();

        match authenticate(username, password) {
            Ok((uid, uname)) => {
                println!("Welcome, {}!", uname);
                match create_session(uid, uname) {
                    Ok(token) => {
                        println!("Session created. Token: {}", token);
                        // In Milestone 2/3, we just wait for the session to end
                        // In a real system, the Login Manager might stay active or switch VT
                        wait_for_session();
                    },
                    Err(e) => println!("Failed to create session: {}", e),
                }
            },
            Err(e) => println!("Login failed: {}", e),
        }
    }
}

fn authenticate(username: &str, password: &str) -> Result<(u32, String), String> {
    let mut client = AipcClient::connect(AUTH_SOCKET_PATH).map_err(|e| e.to_string())?;

    let res = client.request("login_manager", None, AipcMessage::Auth(AuthRequest::Login {
        username: username.to_string(),
        password: password.to_string(),
    })).map_err(|e| e.to_string())?;

    match res {
        AipcMessage::AuthRes(AuthResponse::Authenticated { uid, username }) => Ok((uid, username)),
        AipcMessage::AuthRes(AuthResponse::Error(e)) => Err(e),
        _ => Err("Invalid response from auth service".to_string()),
    }
}

fn create_session(uid: u32, username: String) -> Result<String, String> {
    let mut client = AipcClient::connect(SESSION_SOCKET_PATH).map_err(|e| e.to_string())?;

    let res = client.request("login_manager", None, AipcMessage::Session(SessionRequest::CreateSession {
        uid,
        username,
    })).map_err(|e| e.to_string())?;

    match res {
        AipcMessage::SessionRes(SessionResponse::Success { token }) => Ok(token),
        AipcMessage::SessionRes(SessionResponse::Error(e)) => Err(e),
        _ => Err("Invalid response from session manager".to_string()),
    }
}

fn wait_for_session() {
    // For now, we just wait a bit or could monitor the session
    // In AyuxOS, the Session Manager launches the shell
    println!("Session active. (Press Enter to logout)");
    let mut dummy = String::new();
    let _ = io::stdin().read_line(&mut dummy);
}
