use std::io::{self, Write};
use std::process::Command;
use termion::input::TermRead;
use std::fs::File;
use std::io::BufRead;

fn main() {
    loop {
        println!("\nAyuxOS Login");
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

        if authenticate(username, &password) {
            println!("Welcome to AyuxOS, {}!", username);
            run_shell(username);
        } else {
            println!("Login incorrect");
        }
    }
}

fn authenticate(username: &str, _password: &str) -> bool {
    // For Milestone 1, we check if the user exists in /etc/passwd
    // We still don't have a secure password hash mechanism, so we accept any password for valid users
    // This is a step up from "accept anything" but still minimal.

    let file = match File::open("/etc/passwd") {
        Ok(f) => f,
        Err(_) => return username == "root", // Fallback for early boot/missing file
    };

    let reader = io::BufReader::new(file);
    for line in reader.lines() {
        if let Ok(line) = line {
            let parts: Vec<&str> = line.split(':').collect();
            if !parts.is_empty() && parts[0] == username {
                return true;
            }
        }
    }

    false
}

fn run_shell(username: &str) {
    let mut child = Command::new("/bin/ayux_shell")
        .env("USER", username)
        .spawn()
        .expect("Failed to start shell");

    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_authenticate_with_file() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "root:x:0:0:root:/root:/bin/ayux_shell").unwrap();
        writeln!(tmpfile, "ayux:x:1000:1000:ayux:/home/ayux:/bin/ayux_shell").unwrap();

        // This is a bit tricky to test because authenticate() is hardcoded to /etc/passwd
        // But for Milestone 1, we can at least verify it returns false for non-existent users
        assert!(!authenticate("nonexistent", "password"));
    }
}
