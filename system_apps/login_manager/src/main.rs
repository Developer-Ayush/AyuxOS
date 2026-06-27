use std::io::{self, Write};
use termion::input::TermRead;
use libaipc::{AipcClient, AipcMessage, AuthRequest, AuthResponse, SessionRequest, SessionResponse};

const AUTH_SOCKET_PATH: &str = "/run/auth.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

fn main() {
    loop {
        println!("\nAyuxOS Login (Milestone 2)");
        print!("Username: ");
        io::stdout().flush().unwrap();

        let mut username = String::new();
        io::stdin().read_line(&mut username).unwrap();
        let username = username.trim();

        if username.is_empty() {
            continue;
        }

        print!("Password: ");
        io::stdout().flush().unwrap();

        let password = io::stdin().read_passwd(&mut io::stdout()).unwrap().unwrap_or_default();
        println!();

        match authenticate(username, &password) {
            Ok((uid, uname)) => {
                println!("Welcome to AyuxOS, {}!", uname);
                if let Err(e) = create_session(uid, &uname) {
                    println!("Failed to create session: {}", e);
                }
            },
            Err(e) => {
                println!("Login incorrect: {}", e);
            }
        }
    }
}

fn authenticate(username: &str, password: &str) -> Result<(u32, String), String> {
    let mut client = AipcClient::connect(AUTH_SOCKET_PATH)
        .map_err(|e| format!("Failed to connect to auth service: {}", e))?;

    client.send_message(&AipcMessage::Auth(AuthRequest::Login {
        username: username.to_string(),
        password: password.to_string(),
    })).map_err(|e| format!("Failed to send auth request: {}", e))?;

    match client.receive_message() {
        Ok(AipcMessage::AuthRes(AuthResponse::Authenticated { uid, username })) => Ok((uid, username)),
        Ok(AipcMessage::AuthRes(AuthResponse::Error(e))) => Err(e),
        _ => Err("Received unexpected response from auth service".to_string()),
    }
}

fn create_session(uid: u32, username: &str) -> Result<(), String> {
    let mut client = AipcClient::connect(SESSION_SOCKET_PATH)
        .map_err(|e| format!("Failed to connect to session manager: {}", e))?;

    client.send_message(&AipcMessage::Session(SessionRequest::CreateSession {
        uid,
        username: username.to_string(),
    })).map_err(|e| format!("Failed to send session request: {}", e))?;

    match client.receive_message() {
        Ok(AipcMessage::SessionRes(SessionResponse::Success { token: _ })) => {
            // In Milestone 2, the Session Manager launches the shell.
            // The Login Manager doesn't need to do anything else.
            Ok(())
        },
        Ok(AipcMessage::SessionRes(SessionResponse::Error(e))) => Err(e),
        _ => Err("Received unexpected response from session manager".to_string()),
    }
}
