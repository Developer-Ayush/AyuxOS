use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;

pub struct AipcClient {
    stream: UnixStream,
}

impl AipcClient {
    pub fn connect(path: &str) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        Ok(Self { stream })
    }

    pub fn send_message(&mut self, msg: &str) -> io::Result<()> {
        self.stream.write_all(msg.as_bytes())?;
        Ok(())
    }

    pub fn receive_message(&mut self) -> io::Result<String> {
        let mut buffer = [0u8; 1024];
        let n = self.stream.read(&mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer[..n]).into_owned())
    }
}

pub fn create_listener(path: &str) -> io::Result<std::os::unix::net::UnixListener> {
    if std::path::Path::new(path).exists() {
        std::fs::remove_file(path)?;
    }
    std::os::unix::net::UnixListener::bind(path)
}
