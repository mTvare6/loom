use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use crate::{Request, Response};

pub struct IpcClient {
    stream: BufReader<UnixStream>,
}

impl IpcClient {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            stream: BufReader::new(UnixStream::connect(path)?),
        })
    }

    pub fn send(&mut self, request: Request) -> std::io::Result<Response> {
        let mut msg = serde_json::to_string(&request)?;
        msg.push('\n');
        self.stream.get_ref().write_all(msg.as_bytes())?;
        self.stream.get_ref().flush()?;

        let mut line = String::new();
        self.stream.read_line(&mut line)?;
        if line.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "server closed connection",
            ))
        } else {
            let response = serde_json::from_str(&line)?;
            Ok(response)
        }
    }
}
