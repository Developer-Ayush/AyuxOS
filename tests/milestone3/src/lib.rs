#[cfg(test)]
mod tests {
    use libaipc::{AipcMessage, AipcEnvelope, AipcHeader, MessageType, AIPC_VERSION, AuthRequest};
    use libayux_hal::clock::{Clock, LinuxClock};
    use libayux_hal::random::{Random, LinuxRandom};

    #[test]
    fn test_aipc_serialization() {
        let envelope = AipcEnvelope {
            header: AipcHeader {
                version: AIPC_VERSION,
                message_type: MessageType::Request,
                sender: "test".to_string(),
                session_id: Some("session-123".to_string()),
                correlation_id: 42,
            },
            message: AipcMessage::Auth(AuthRequest::ListUsers),
        };

        let encoded = bincode::serialize(&envelope).unwrap();
        let decoded: AipcEnvelope = bincode::deserialize(&encoded).unwrap();

        assert_eq!(decoded.header.version, AIPC_VERSION);
        assert_eq!(decoded.header.correlation_id, 42);
        assert_eq!(decoded.header.sender, "test");
        if let AipcMessage::Auth(AuthRequest::ListUsers) = decoded.message {
            // Success
        } else {
            panic!("Decoded message mismatch");
        }
    }

    #[test]
    fn test_hal_clock() {
        let clock = LinuxClock;
        let time = clock.get_time().unwrap();
        assert!(time > 0);
    }

    #[test]
    fn test_hal_random() {
        let rnd = LinuxRandom;
        let mut buf = [0u8; 16];
        // Note: this might fail if /dev/urandom is not available in the test environment
        if let Ok(_) = rnd.get_random(&mut buf) {
             assert_ne!(buf, [0u8; 16]);
        }
    }
}
