use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use crate::{Event, Request, Response, ServerMsg};

pub struct IpcClient {
    stream: BufReader<UnixStream>,
    pending_line: String,
    pending_event: Option<Event>,
}

impl IpcClient {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        Ok(Self {
            stream: BufReader::new(stream),
            pending_line: String::new(),
            pending_event: None,
        })
    }

    pub fn send(&mut self, request: Request) -> io::Result<Response> {
        let mut msg = serde_json::to_string(&request)?;
        msg.push('\n');
        self.stream.get_ref().write_all(msg.as_bytes())?;
        self.stream.get_ref().flush()?;

        loop {
            match self.read_frame(Duration::from_secs(2))? {
                ServerMsg::Response(response) => return Ok(response),
                ServerMsg::Event(event) => {
                    self.pending_event = Some(event);
                }
            }
        }
    }

    pub fn try_recv_event(&mut self, timeout: Duration) -> io::Result<Event> {
        if let Some(event) = self.pending_event.take() {
            return Ok(event);
        }

        match self.read_frame(timeout)? {
            ServerMsg::Event(event) => Ok(event),
            ServerMsg::Response(_) => Err(io::Error::new(
                ErrorKind::WouldBlock,
                "a stale Response arrived instead of an Event",
            )),
        }
    }

    fn read_frame(&mut self, timeout: Duration) -> io::Result<ServerMsg> {
        self.stream.get_ref().set_read_timeout(Some(timeout))?;

        loop {
            match self.stream.read_line(&mut self.pending_line) {
                Ok(0) => {
                    return Err(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "server closed connection",
                    ));
                }
                Ok(_) => {
                    let frame = serde_json::from_str::<ServerMsg>(&self.pending_line);
                    self.pending_line.clear();
                    return frame.map_err(io::Error::from);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    return Err(e);
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }
}
