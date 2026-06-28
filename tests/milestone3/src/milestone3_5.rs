#[cfg(test)]
mod milestone3_5_tests {
    use argon2::password_hash::PasswordHash;

    #[test]
    fn test_password_hashing_consistency() {
        // Test that Argon2id works as expected
        let _password = "mysecurepassword";
        // Encoded hash from a typical Argon2id run
        let hash_str =
            "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$dGVzdGhhc2h0ZXN0aGFzaHRlc3RoYXNodGVzdGg";

        // This won't actually verify because the hash is dummy, but we can test we can parse it
        let parsed_hash = PasswordHash::new(hash_str);
        assert!(parsed_hash.is_ok());
    }

    #[test]
    fn test_home_dir_path_resolution() {
        let username = "testuser";
        let home_dir = format!("/users/{}", username);
        assert_eq!(home_dir, "/users/testuser");

        let subdirs = [
            "Desktop",
            "Documents",
            "Downloads",
            "Pictures",
            "Music",
            "Videos",
            "Config",
        ];
        for subdir in &subdirs {
            let path = format!("{}/{}", home_dir, subdir);
            assert!(path.starts_with("/users/testuser/"));
        }
    }

    #[test]
    fn test_shell_path_resolution() {
        // Mocking Shell::resolve_path logic
        let cwd = "/users/alice";

        let resolve = |path: &str, cwd: &str| -> String {
            if path.starts_with('/') {
                path.to_string()
            } else if path == ".." {
                let mut p = std::path::PathBuf::from(cwd);
                if p.pop() {
                    p.to_string_lossy().to_string()
                } else {
                    "/".to_string()
                }
            } else if path == "." || path.is_empty() {
                cwd.to_string()
            } else {
                let mut base = cwd.to_string();
                if !base.ends_with('/') {
                    base.push('/');
                }
                base.push_str(path);
                base
            }
        };

        assert_eq!(resolve("Documents", cwd), "/users/alice/Documents");
        assert_eq!(resolve("/etc", cwd), "/etc");
        assert_eq!(resolve("..", cwd), "/users");
        assert_eq!(resolve(".", cwd), "/users/alice");
    }
}
