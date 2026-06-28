use libaipc::{
    AipcClient, AipcMessage, AuthRequest, AuthResponse, LogLevel, SessionRequest, SessionResponse,
};
use libayux::ayux_log;
use std::io::{self, Write};

const AUTH_SOCKET_PATH: &str = "/run/auth.sock";
const SESSION_SOCKET_PATH: &str = "/run/session.sock";

fn main() {
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
        let password = read_password();

        match authenticate(username, &password) {
            Ok((uid, uname, role, caps)) => {
                ayux_log(
                    LogLevel::Info,
                    "login_manager",
                    &format!("Login success: {}", uname),
                );
                println!("\nWelcome, {}!\n", uname);
                match create_session(uid, uname, role, caps) {
                    Ok(token) => {
                        wait_for_session(&token);
                    }
                    Err(e) => println!("\nError: Failed to create session: {}\n", e),
                }
            }
            Err(e) => {
                ayux_log(
                    LogLevel::Warn,
                    "login_manager",
                    &format!("Login failure for {}: {}", username, e),
                );
                println!("\nAuthentication failed.");
                println!("{}\n", e);
            }
        }
    }
}

fn get_user_count() -> Result<usize, String> {
    let mut client = AipcClient::connect(AUTH_SOCKET_PATH).map_err(|e| e.to_string())?;

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
    println!("\n========================================");
    println!("AyuxOS Setup");
    println!("====================");
    println!("No administrator account exists.");
    println!("Please create the first administrator.\n");

    loop {
        let username = prompt("Username:         ");
        if username.is_empty() {
            continue;
        }

        let display_name = prompt("Display Name:     ");
        if display_name.is_empty() {
            continue;
        }

        println!("Password: ");
        let password = read_password();
        if password.is_empty() {
            continue;
        }

        println!("Confirm Password: ");
        let confirm_password = read_password();
        if password != confirm_password {
            println!("\nError: Passwords do not match. Please try again.\n");
            continue;
        }

        match create_user(&username, &password, &display_name, "Administrator") {
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
    print!("{}", label);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
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
) -> Result<(), String> {
    let mut client = AipcClient::connect(AUTH_SOCKET_PATH).map_err(|e| e.to_string())?;

    let res = client
        .request(
            "login_manager",
            None,
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
) -> Result<(u32, String, String, Vec<String>), String> {
    let mut client = AipcClient::connect(AUTH_SOCKET_PATH).map_err(|e| e.to_string())?;

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
            uid,
            username,
            role,
            capabilities,
        }) => Ok((uid, username, role, capabilities)),
        AipcMessage::AuthRes(AuthResponse::Error(e)) => Err(e),
        _ => Err("Invalid response from auth service".to_string()),
    }
}

fn create_session(
    uid: u32,
    username: String,
    role: String,
    capabilities: Vec<String>,
) -> Result<String, String> {
    let mut client = AipcClient::connect(SESSION_SOCKET_PATH).map_err(|e| e.to_string())?;

    // We need to update SessionRequest to include role and capabilities
    // Or we update Session Manager to fetch them.
    // Requirement 4.2 in set_plan said: Update `CreateSession` to retrieve user details from `auth_service`.
    // But currently I'm passing them from login_manager because I updated AuthResponse.

    // Let's update libaipc first to include role and capabilities in SessionRequest::CreateSession

    let res = client
        .request(
            "login_manager",
            None,
            AipcMessage::Session(SessionRequest::CreateSession {
                uid,
                username,
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
    // For now, we just wait for the session to end by checking if it's still valid
    // In a real system, we might use a more efficient notification mechanism
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mut client = match AipcClient::connect(SESSION_SOCKET_PATH) {
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
