use libaipc::{
    AipcClient, AipcMessage, AuthRequest, AuthResponse, LogLevel, SessionRequest, SessionResponse,
};
use libayux::ayux_log;
use libayux::paths;
use std::io;

fn main() {
    libayux::setup_env();

    // Quiet start
    match get_user_count() {
        Ok(0) => {
            run_setup_wizard();
        }
        Ok(_) => {}
        Err(e) => {
            println!("Error checking user count: {}. Proceeding to login.", e);
        }
    }

    loop {
        println!("Username:");
        let mut username = String::new();
        if io::stdin().read_line(&mut username).is_err() {
            continue;
        }
        let username = username.trim();

        if username.is_empty() {
            continue;
        }

        println!("Password:");
        let password = read_password();

        match authenticate(username, &password) {
            Ok((internal_id, uname, display_name, role, caps)) => {
                ayux_log(
                    LogLevel::Info,
                    "login_manager",
                    &format!("Login success for internal_id: {}", internal_id),
                );
                // Clear screen for a fresh session
                print!("\x1B[2J\x1B[1;1H");
                println!("\nWelcome, {}!\n", display_name);
                match create_session(internal_id, uname, display_name, role, caps) {
                    Ok(token) => {
                        wait_for_session(&token);
                    }
                    Err(e) => println!("\nError: Failed to create session: {}\n", e),
                }
            }
            Err(_) => {
                ayux_log(
                    LogLevel::Warn,
                    "login_manager",
                    &format!("Login failure for {}", username),
                );
                println!("\nAuthentication failed.\n");
            }
        }
    }
}

fn get_user_count() -> Result<usize, String> {
    let mut client = AipcClient::connect(paths::AUTH_SOCKET).map_err(|e| e.to_string())?;

    let res = client
        .request(
            "login_manager",
            None,
            AipcMessage::Auth(AuthRequest::CountUsers),
        )
        .map_err(|e| e.to_string())?;

    match res {
        AipcMessage::AuthRes(AuthResponse::UserCount(count)) => Ok(count),
        _ => Err("Invalid response from auth service".to_string()),
    }
}

fn run_setup_wizard() {
    libayux::print_heading("AyuxOS Setup");
    println!("No administrator account exists.");
    println!("Please create the first administrator.\n");

    loop {
        let username = prompt("Username:");
        if username.is_empty() {
            continue;
        }

        if let Err(e) = libayux::validate_username(&username) {
            println!("\nError: {}\n", e);
            continue;
        }

        let display_name = prompt("Display Name:");
        if display_name.is_empty() {
            continue;
        }

        println!("Password:");
        let password = read_password();
        if password.is_empty() {
            continue;
        }

        println!("Confirm Password:");
        let confirm_password = read_password();
        if password != confirm_password {
            println!("\nError: Passwords do not match. Please try again.\n");
            continue;
        }

        match create_user(&username, &password, &display_name, "Administrator", None) {
            Ok(_) => {
                println!("\nAdministrator account created successfully.");
                println!("Proceeding to login...\n");
                break;
            }
            Err(e) => println!("\nError: Failed to create user: {}\n", e),
        }
    }
}

fn prompt(label: &str) -> String {
    println!("{}", label);
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return String::new();
    }
    input.trim().to_string()
}

fn read_password() -> String {
    use std::io::stdin;
    use termion::input::TermRead;

    let mut password = String::new();
    if let Ok(Some(p)) = stdin().read_passwd(&mut io::stdout()) {
        password = p;
    }
    password
}

fn create_user(
    username: &str,
    password: &str,
    display_name: &str,
    role: &str,
    session_token: Option<String>,
) -> Result<(), String> {
    let mut client = AipcClient::connect(paths::AUTH_SOCKET).map_err(|e| e.to_string())?;

    let res = client
        .request(
            "login_manager",
            session_token,
            AipcMessage::Auth(AuthRequest::CreateUser {
                username: username.to_string(),
                password: password.to_string(),
                display_name: display_name.to_string(),
                role: role.to_string(),
            }),
        )
        .map_err(|e| e.to_string())?;

    match res {
        AipcMessage::AuthRes(AuthResponse::Success) => Ok(()),
        AipcMessage::AuthRes(AuthResponse::Error(e)) => Err(e),
        _ => Err("Invalid response from auth service".to_string()),
    }
}

fn authenticate(
    username: &str,
    password: &str,
) -> Result<(String, String, String, String, Vec<String>), String> {
    let mut client = AipcClient::connect(paths::AUTH_SOCKET).map_err(|e| e.to_string())?;

    let res = client
        .request(
            "login_manager",
            None,
            AipcMessage::Auth(AuthRequest::Login {
                username: username.to_string(),
                password: password.to_string(),
            }),
        )
        .map_err(|e| e.to_string())?;

    match res {
        AipcMessage::AuthRes(AuthResponse::Authenticated {
            internal_id,
            username,
            display_name,
            role,
            capabilities,
        }) => Ok((internal_id, username, display_name, role, capabilities)),
        AipcMessage::AuthRes(AuthResponse::Error(e)) => Err(e),
        _ => Err("Invalid response from auth service".to_string()),
    }
}

fn create_session(
    internal_id: String,
    username: String,
    display_name: String,
    role: String,
    capabilities: Vec<String>,
) -> Result<String, String> {
    let mut client = AipcClient::connect(paths::SESSION_SOCKET).map_err(|e| e.to_string())?;

    let res = client
        .request(
            "login_manager",
            None,
            AipcMessage::Session(SessionRequest::CreateSession {
                internal_id,
                username,
                display_name,
                role,
                capabilities,
            }),
        )
        .map_err(|e| e.to_string())?;

    match res {
        AipcMessage::SessionRes(SessionResponse::Success { token }) => Ok(token),
        AipcMessage::SessionRes(SessionResponse::Error(e)) => Err(e),
        _ => Err("Invalid response from session manager".to_string()),
    }
}

fn wait_for_session(token: &str) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mut client = match AipcClient::connect(paths::SESSION_SOCKET) {
            Ok(c) => c,
            Err(_) => break,
        };

        let res = client.request(
            "login_manager",
            None,
            AipcMessage::Session(SessionRequest::ValidateSession {
                token: token.to_string(),
            }),
        );
        match res {
            Ok(AipcMessage::SessionRes(SessionResponse::Valid { .. })) => continue,
            _ => break,
        }
    }
    println!("Session ended.");
}
