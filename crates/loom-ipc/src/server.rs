use std::fs;
use std::io::ErrorKind;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
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

    pub fn run_until<F: Fn(Request) -> Response + Send + Sync + 'static, S: Fn() -> bool>(
        &self,
        f: Arc<F>,
        should_stop: S,
    ) -> std::io::Result<()> {
        match fs::remove_file(&self.socket) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        if let Some(parent) = self.socket.parent() {
            fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&self.socket)?;

        // Generally it blocks the thread waiting for another
        // Having it be nonblocking allows should_stop to be run exiting
        // run_until and allowing audio thread to be dropped and exit gracefully
        listener.set_nonblocking(true)?;

        info!("IPC Server listening on {:?}", self.socket);

        while !should_stop() {
            match listener.accept() {
                Ok((stream, _address)) => {
                    let handler = f.clone();
                    // Each client is handled in it's thread
                    thread::spawn(move || {
                        if let Err(error) = handle_client(stream, handler) {
                            error!("IPC client error: {}", error);
                        }
                    });
                }
                // Would've been blocking at this point but explicitly disabled
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
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
