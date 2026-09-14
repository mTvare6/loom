use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

use crate::{Request, Response};

pub struct IpcServer {
    socket: PathBuf,
}

impl IpcServer {
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            socket: socket_path.as_ref().to_path_buf(),
        }
    }

    pub fn run<F: Fn(Request) -> Response + Send + Sync + 'static>(
        &self,
        f: Arc<F>,
    ) -> std::io::Result<()> {
        match fs::remove_file(&self.socket) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        let listener = UnixListener::bind(&self.socket)?;
        info!("IPC Server listening on {:?}", self.socket);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handler = f.clone();
                    thread::spawn(move || {
                        if let Err(error) = handle_client(stream, handler) {
                            error!("IPC client error: {}", error);
                        }
                    });
                }
                Err(error) => error!("Failed to accept client: {}", error),
            }
        }

        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
    }
}

fn handle_client<F: Fn(Request) -> Response + Send + Sync + 'static>(
    mut stream: UnixStream,
    handler: Arc<F>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Ok(());
        }

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => handler(request),
            Err(error) => {
                error!("IPC parse error: {}", error);
                Response::Error
            }
        };

        let mut response_str = serde_json::to_string(&response).map_err(std::io::Error::other)?;
        response_str.push('\n');
        stream.write_all(response_str.as_bytes())?;
    }
}
