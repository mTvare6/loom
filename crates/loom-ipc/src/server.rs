use std::fs;
use std::io::ErrorKind;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{error, info};

use crate::{Event, Frame, Request, Response};

pub struct EventNotifier {
    inner: Mutex<(u64, Option<Event>)>,
}

impl EventNotifier {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new((0, None)),
        }
    }

    pub fn notify(&self, event: Event) {
        let mut guard = self.inner.lock().unwrap();
        guard.0 += 1;
        guard.1 = Some(event);
    }
}

pub struct IpcServer {
    socket: PathBuf,
    event_notifier: Arc<EventNotifier>,
}

impl IpcServer {
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            socket: socket_path.as_ref().to_path_buf(),
            event_notifier: Arc::new(EventNotifier::new()),
        }
    }

    pub fn notifier(&self) -> Arc<EventNotifier> {
        self.event_notifier.clone()
    }

    pub fn run_until<
        F: Fn(Request) -> (Response, Option<Event>) + Send + Sync + 'static,
        S: Fn() -> bool,
    >(
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
        listener.set_nonblocking(true)?;

        info!("IPC Server listening on {:?}", self.socket);

        while !should_stop() {
            match listener.accept() {
                Ok((stream, _address)) => {
                    let handler = f.clone();
                    let event_notifier = self.event_notifier.clone();

                    thread::spawn(move || {
                        if let Err(error) = handle_client(stream, event_notifier, handler) {
                            error!("IPC client error: {}", error);
                        }
                    });
                }
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

fn handle_client<F: Fn(Request) -> (Response, Option<Event>) + Send + Sync + 'static>(
    mut stream: UnixStream,
    notifier: Arc<EventNotifier>,
    handler: Arc<F>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(20)))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    let mut last_seen_version = 0;

    loop {
        let (current_version, maybe_event) = {
            let guard = notifier.inner.lock().unwrap();
            (guard.0, guard.1.clone())
        };

        if current_version > last_seen_version {
            last_seen_version = current_version;

            if let Some(event) = maybe_event {
                let mut payload =
                    serde_json::to_string(&Frame::Event(event)).map_err(std::io::Error::other)?;
                payload.push('\n');
                if stream.write_all(payload.as_bytes()).is_err() {
                    return Ok(());
                }
            }
        }

        match reader.read_line(&mut line) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                let response = match serde_json::from_str::<Request>(&line) {
                    Ok(request) => {
                        let (response, maybe_event) = handler(request);
                        if let Some(event) = maybe_event {
                            notifier.notify(event);
                        }

                        response
                    }
                    Err(error) => {
                        error!("IPC parse error: {}", error);
                        Response::Error
                    }
                };

                line.clear();

                let mut response_str = serde_json::to_string(&Frame::Response(response))
                    .map_err(std::io::Error::other)?;
                response_str.push('\n');
                stream.write_all(response_str.as_bytes())?;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {}
            Err(e) => return Err(e),
        }
    }
}
