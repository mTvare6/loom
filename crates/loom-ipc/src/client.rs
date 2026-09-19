use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use crate::{Event, Frame, Request, Response};

pub struct IpcClient {
    stream: BufReader<UnixStream>,
    in_line: String,
    pending_events: VecDeque<Event>,
}

impl IpcClient {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        Ok(Self {
            stream: BufReader::new(stream),
            in_line: String::new(),
            pending_events: VecDeque::new(),
        })
    }

    pub fn send(&mut self, request: Request) -> io::Result<Response> {
        self.stream
            .get_ref()
            .set_read_timeout(Some(Duration::from_secs(2)))?;

        let mut msg = serde_json::to_string(&request)?;
        msg.push('\n');

        self.stream.get_ref().write_all(msg.as_bytes())?;
        self.stream.get_ref().flush()?;

        loop {
            self.in_line.clear();
            let bytes_read = self.stream.read_line(&mut self.in_line)?;

            if bytes_read == 0 {
                return Err(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "server closed connection",
                ));
            }

            match serde_json::from_str::<Frame>(&self.in_line) {
                Ok(Frame::Response(response)) => return Ok(response),
                Ok(Frame::Event(event)) => {
                    self.pending_events.push_back(event);
                }
                Err(err) => {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("Failed to parse frame: {err}"),
                    ));
                }
            }
        }
    }

    pub fn try_recv_event(&mut self, timeout: Duration) -> io::Result<Event> {
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(event);
        }

        self.stream.get_ref().set_read_timeout(Some(timeout))?;

        self.in_line.clear();
        match self.stream.read_line(&mut self.in_line) {
            Ok(0) => Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "server closed connection",
            )),
            Ok(_) => match serde_json::from_str::<Frame>(&self.in_line) {
                Ok(Frame::Event(event)) => Ok(event),
                Ok(Frame::Response(_)) => Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "unexpected response frame while reading events",
                )),
                Err(err) => Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Failed to parse frame: {err}"),
                )),
            },
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                Err(io::Error::new(ErrorKind::WouldBlock, "no events ready"))
            }
            Err(e) => Err(e),
        }
    }
}
