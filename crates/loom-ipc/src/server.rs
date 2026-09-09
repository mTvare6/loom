use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

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

    // TODO: Handle errors
    pub fn run<F: Fn(Request) -> Response + Send + Sync + 'static>(&self, f: Arc<F>) {
        let _ = fs::remove_file(&self.socket);
        let listener = UnixListener::bind(&self.socket).expect("Failed to bind socket");
        println!("IPC Server listening on {:?}", self.socket);

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let handler = f.clone();
                    thread::spawn(move || {
                        let mut reader = BufReader::new(stream.try_clone().unwrap()); // boom
                        let mut line = String::new();

                        while let Ok(bytes_read) = reader.read_line(&mut line) {
                            if bytes_read == 0 {
                                break;
                            }

                            match serde_json::from_str::<Request>(&line) {
                                Ok(req) => {
                                    let response = handler(req);

                                    if let Ok(mut response_str) = serde_json::to_string(&response) {
                                        response_str.push('\n');
                                        let _ = stream.write_all(response_str.as_bytes());
                                    }
                                }
                                Err(e) => {
                                    eprintln!("IPC Parse error: {}", e);
                                    let err_response = Response::Error;
                                    let mut err_str = serde_json::to_string(&err_response).unwrap(); // boom
                                    err_str.push('\n');
                                    let _ = stream.write_all(err_str.as_bytes());
                                }
                            }

                            line.clear();
                        }
                    });
                }
                Err(e) => eprintln!("Failed to accept client: {}", e),
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
    }
}
