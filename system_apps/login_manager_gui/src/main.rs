use libui::{Window, widgets::{Button, Label, Panel, TextBox, PasswordBox}};
use libgraphics::{Rect};
use libayux::paths;
use std::thread;
use std::time::Duration;
use libaipc::{AipcClient, AipcMessage, AuthRequest, AuthResponse, SessionRequest, SessionResponse};
use std::sync::{Arc, Mutex};
use std::process::Command;

fn main() {
    libayux::setup_env();

    let mut window = Window::new("AyuxOS Login", 400, 300).expect("Failed to create login window");

    window.add_widget(Box::new(Panel {
        rect: Rect::new(50, 50, 300, 200),
    }));

    window.add_widget(Box::new(Label {
        text: "Username:".to_string(),
        rect: Rect::new(70, 70, 100, 20),
    }));

    let user_input = Arc::new(Mutex::new(String::new()));
    let user_input_widget = TextBox {
        text: Arc::clone(&user_input),
        rect: Rect::new(170, 70, 150, 30),
        focused: true,
        shift_pressed: false,
        caps_lock: false,
        caret_visible: std::cell::Cell::new(true),
        last_blink: std::cell::Cell::new(std::time::Instant::now()),
    };

    window.add_widget(Box::new(user_input_widget));

    window.add_widget(Box::new(Label {
        text: "Password:".to_string(),
        rect: Rect::new(70, 120, 100, 20),
    }));

    let pass_input = Arc::new(Mutex::new(String::new()));
    let pass_input_widget = PasswordBox {
        text: Arc::clone(&pass_input),
        rect: Rect::new(170, 120, 150, 30),
        focused: false,
        shift_pressed: false,
        caps_lock: false,
        caret_visible: std::cell::Cell::new(true),
        last_blink: std::cell::Cell::new(std::time::Instant::now()),
    };

    window.add_widget(Box::new(pass_input_widget));

    let login_status = Arc::new(Mutex::new(None));
    let login_status_clone = Arc::clone(&login_status);
    let user_input_clone = Arc::clone(&user_input);
    let pass_input_clone = Arc::clone(&pass_input);

    let login_button = Button {
        text: "Login".to_string(),
        rect: Rect::new(150, 200, 100, 40),
        pressed: false,
        on_click: Some(Box::new(move || {
            let username = user_input_clone.lock().unwrap().clone();
            let password = pass_input_clone.lock().unwrap().clone();

            if let Ok(mut client) = AipcClient::connect(paths::AUTH_SOCKET) {
                let resp = client.request("login_manager_gui", None, AipcMessage::Auth(AuthRequest::Login {
                    username: username.clone(),
                    password,
                }));

                match resp {
                    Ok(AipcMessage::AuthRes(AuthResponse::Authenticated { internal_id, username, display_name, role, capabilities })) => {
                        // Success! Create session
                        if let Ok(mut s_client) = AipcClient::connect(paths::SESSION_SOCKET) {
                            let s_resp = s_client.request("login_manager_gui", None, AipcMessage::Session(SessionRequest::CreateSession {
                                internal_id, username, display_name, role, capabilities
                            }));
                            if let Ok(AipcMessage::SessionRes(SessionResponse::Success { .. })) = s_resp {
                                *login_status_clone.lock().unwrap() = Some(true);
                            } else {
                                *login_status_clone.lock().unwrap() = Some(false);
                            }
                        }
                    }
                    _ => {
                        *login_status_clone.lock().unwrap() = Some(false);
                    }
                }
            }
        })),
    };

    window.add_widget(Box::new(login_button));

    loop {
        window.render();

        let status = *login_status.lock().unwrap();
        if status == Some(true) {
            println!("Login successful! Launching desktop...");
            Command::new(paths::app_executable("desktop")).spawn().ok();
            break;
        } else if status == Some(false) {
            // Display error?
            *login_status.lock().unwrap() = None;
        }

        thread::sleep(Duration::from_millis(32));
    }
}
