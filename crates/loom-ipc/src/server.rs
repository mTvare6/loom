use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use mio::net::UnixStream as MioUnixStream;
use mio::{Events, Interest, Poll, Token, Waker};
use tracing::{error, info};

use crate::{Event, Request, Response, ServerMsg};

type ClientId = u64;
const SOCKET: Token = Token(0);
const EVENT_TOKEN: Token = Token(1);

pub struct EventNotifier {
    inner: Mutex<(u64, Option<Event>)>,
    wakers: Mutex<HashMap<ClientId, Arc<Waker>>>,
    next_id: AtomicU64,
}

impl EventNotifier {
    fn new() -> Self {
        Self {
            inner: Mutex::new((0, None)),
            wakers: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
        }
    }

    fn register(&self, waker: Arc<Waker>) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.wakers.lock().unwrap().insert(id, waker);
        id
    }

    fn unregister(&self, id: ClientId) {
        self.wakers.lock().unwrap().remove(&id);
    }

    pub fn publish_from<F: Fn() -> (Response, Option<Event>)>(&self, f: F) -> Response {
        let (result, published) = {
            let mut state = self.inner.lock().unwrap();
            let (r, maybe_event) = f();
            if let Some(event) = maybe_event {
                state.0 += 1;
                state.1 = Some(event);
            }
            (r, maybe_event.is_some())
        };

        if published {
            for waker in self.wakers.lock().unwrap().values() {
                let _ = waker.wake();
            }
        }

        result
    }

    fn snapshot(&self) -> (u64, Option<Event>) {
        let state = self.inner.lock().unwrap();
        (state.0, state.1)
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
                    stream.set_nonblocking(true)?;
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

fn queue_server_msg(out_buf: &mut Vec<u8>, frame: &ServerMsg) -> io::Result<()> {
    let mut line = serde_json::to_string(frame)?;
    line.push('\n');
    out_buf.extend_from_slice(line.as_bytes());
    Ok(())
}

fn flush(stream: &mut MioUnixStream, out_buf: &mut Vec<u8>) -> io::Result<()> {
    while !out_buf.is_empty() {
        match stream.write(out_buf) {
            Ok(0) => return Err(io::Error::new(ErrorKind::WriteZero, "socket closed")),
            Ok(n) => {
                out_buf.drain(0..n);
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn flush_and_adjust_interest(
    poll: &Poll,
    reader: &mut BufReader<MioUnixStream>,
    out_buf: &mut Vec<u8>,
    interest: &mut Interest,
) -> io::Result<()> {
    match flush(reader.get_mut(), out_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
        Err(e) => return Err(e),
    }
    let new_interest = if out_buf.is_empty() {
        Interest::READABLE
    } else {
        Interest::READABLE | Interest::WRITABLE
    };
    if new_interest != *interest {
        poll.registry()
            .reregister(reader.get_mut(), SOCKET, new_interest)?;
        *interest = new_interest;
    }
    Ok(())
}

fn handle_client<F: Fn(Request) -> (Response, Option<Event>) + Send + Sync + 'static>(
    stream: UnixStream,
    notifier: Arc<EventNotifier>,
    handler: Arc<F>,
) -> io::Result<()> {
    let mut mio_stream = MioUnixStream::from_std(stream);

    let mut poll = Poll::new()?;
    let mut interest = Interest::READABLE;
    poll.registry()
        .register(&mut mio_stream, SOCKET, interest)?;

    let waker = Arc::new(Waker::new(poll.registry(), EVENT_TOKEN)?);
    let client_id = notifier.register(waker);

    let mut reader = BufReader::new(mio_stream);
    let mut in_line = String::new();
    let mut out_buf: Vec<u8> = Vec::new();
    let mut last_seen_version = 0u64;
    let mut events = Events::with_capacity(4);
    let (version, event) = notifier.snapshot();
    if version > last_seen_version {
        last_seen_version = version;
        if let Some(event) = event {
            queue_server_msg(&mut out_buf, &ServerMsg::Event(event))?;
        }
    }

    let result = match flush_and_adjust_interest(&poll, &mut reader, &mut out_buf, &mut interest) {
        Err(e) => Err(e),
        Ok(()) => 'outer: loop {
            if let Err(e) = poll.poll(&mut events, None) {
                if e.kind() == ErrorKind::Interrupted {
                    continue;
                }
                break Err(e);
            }

            let mut client_disconnected = false;

            for ev in events.iter() {
                match ev.token() {
                    EVENT_TOKEN => {
                        let (version, event) = notifier.snapshot();
                        if version > last_seen_version {
                            last_seen_version = version;
                            if let Some(event) = event {
                                if let Err(e) =
                                    queue_server_msg(&mut out_buf, &ServerMsg::Event(event))
                                {
                                    break 'outer Err(e);
                                }
                            }
                        }
                    }
                    SOCKET => loop {
                        match reader.read_line(&mut in_line) {
                            Ok(0) => {
                                client_disconnected = true;
                                break;
                            }
                            Ok(_) => {
                                let response = match serde_json::from_str::<Request>(&in_line) {
                                    Ok(request) => notifier.publish_from(|| handler(request)),
                                    Err(error) => {
                                        error!("IPC parse error: {}", error);
                                        Response::Error
                                    }
                                };
                                in_line.clear();
                                if let Err(e) =
                                    queue_server_msg(&mut out_buf, &ServerMsg::Response(response))
                                {
                                    break 'outer Err(e);
                                }
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                            Err(e) => break 'outer Err(e),
                        }
                    },
                    _ => unreachable!("no other token is ever registered"),
                }
            }

            if let Err(e) =
                flush_and_adjust_interest(&poll, &mut reader, &mut out_buf, &mut interest)
            {
                break Err(e);
            }

            if client_disconnected && out_buf.is_empty() {
                break Ok(());
            }
        },
    };

    notifier.unregister(client_id);
    result
}
