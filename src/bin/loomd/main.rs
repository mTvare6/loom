// SPDX-License-Identifier: MPL-2.0

mod state;

use loom_ipc::IpcServer;
use state::AudioState;
use std::{sync::Arc, thread::JoinHandle};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;

struct AudioThread {
    shutdown_tx: loom_pipewire::ShutdownTransmitter,
    thread: Option<JoinHandle<()>>,
}

impl AudioThread {
    fn start(state: Arc<AudioState>) -> Self {
        let (shutdown_tx, shutdown_rx) = loom_pipewire::shutdown_channel();
        let thread = std::thread::spawn(move || {
            if let Err(error) = loom_pipewire::run_audio_engine(state, shutdown_rx) {
                error!("Loom audio stopped: {error}");
            }
        });
        Self {
            shutdown_tx,
            thread: Some(thread),
        }
    }
}

impl Drop for AudioThread {
    fn drop(&mut self) {
        self.shutdown_tx.shutdown();
        let Some(thread) = self.thread.take() else {
            return;
        };
        if thread.join().is_err() && !std::thread::panicking() {
            error!("Loom audio thread panicked");
        }
    }
}

// We need to change these paths
fn init_logging() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("/tmp", "loom_daemon.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_thread_ids(true)
        .init();

    guard
}

// TODO: maybe use env variable for the socket
fn main() {
    let _wg = init_logging();

    info!("Starting loom daemon");

    let shared_state = Arc::new(AudioState::new(1.0));
    let _audio_thread = AudioThread::start(shared_state.clone());
    let ipc_server = IpcServer::new("/tmp/loom_audio.sock");
    let state = shared_state.clone();
    match ipc_server.run(Arc::new(move |request| state.handle_query(request))) {
        Err(error) => error!("Ipc Server exitted with error: {}", error),
        Ok(()) => {}
    }
}
