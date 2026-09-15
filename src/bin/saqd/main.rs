// SPDX-License-Identifier: MPL-2.0

mod state;

use directories::ProjectDirs;
use saq_ipc::IpcServer;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::flag;
use state::AudioState;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use tracing::{error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;

struct AudioStateStore {
    path: Option<PathBuf>,
}

impl AudioStateStore {
    // TODO: Rewrite it to toml and have the values be human-readable
    // Currently AtomicU32 format is used for volume and semitones
    const STATE_FILE_NAME: &str = "state.json";

    fn new() -> Self {
        let path = ProjectDirs::from("com", "epestr", "saq")
            .map(|dirs| dirs.data_dir().join(Self::STATE_FILE_NAME));
        Self { path }
    }

    fn load(&self) -> AudioState {
        let Some(path) = &self.path else {
            return AudioState::new(1.0);
        };
        if !path.exists() {
            return AudioState::new(1.0);
        }

        match File::open(path)
            .map_err(serde_json::Error::io)
            .and_then(serde_json::from_reader::<_, AudioState>)
        {
            Ok(state) => state,
            Err(error) => {
                warn!(
                    "Could not load audio state from {}: {error}",
                    path.display()
                );
                AudioState::new(1.0)
            }
        }
    }

    fn save(&self, state: &AudioState) {
        let Some(path) = &self.path else {
            return;
        };

        let result = || -> Result<(), Box<dyn std::error::Error>> {
            let Some(parent) = path.parent() else {
                return Err("AudioState path has no parent directory".into());
            };
            fs::create_dir_all(parent)?;

            let temporary = path.with_extension("json.tmp");
            let file = File::create(&temporary)?;

            serde_json::to_writer_pretty(file, state)?;
            fs::rename(temporary, path)?;

            Ok(())
        }();

        if let Err(error) = result {
            error!("Could not save audio state to {}: {error}", path.display());
        }
    }
}

struct AudioThread {
    shutdown_tx: saq_pipewire::ShutdownTransmitter,
    thread: Option<JoinHandle<()>>,
}

impl AudioThread {
    fn start(state: Arc<AudioState>) -> Self {
        let (shutdown_tx, shutdown_rx) = saq_pipewire::shutdown_channel();
        let thread = std::thread::spawn(move || {
            if let Err(error) = saq_pipewire::run_audio_engine(state, shutdown_rx) {
                error!("Śaq audio stopped: {error}");
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
            error!("Śaq audio thread panicked");
        }
    }
}

// We need to change these paths
fn init_logging() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("/tmp", "saqd.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_thread_ids(true)
        .init();

    guard
}

fn main() {
    let _ = init_logging();

    info!("Starting saq daemon");

    let state_store = AudioStateStore::new();
    let shared_state = Arc::new(state_store.load());

    let stop_ipc = Arc::new(AtomicBool::new(false));

    // Sets inner Arc<AtomicBool> to true
    flag::register(SIGINT, stop_ipc.clone()).expect("Couldn't register a SIGINT handler");
    flag::register(SIGTERM, stop_ipc.clone()).expect("Couldn't register a SIGTERM handler");

    let audio_thread = AudioThread::start(shared_state.clone());

    // TODO: Have socket location to be chosen more carefully or be configurable
    let ipc_server = IpcServer::new("/tmp/saq_audio.sock");
    let state = shared_state.clone();

    if let Err(error) = ipc_server
        .run_until(Arc::new(move |request| state.handle_query(request)), || {
            stop_ipc.load(Ordering::Acquire)
        })
    {
        error!("IPC server exited with error: {error}");
    }

    drop(audio_thread);

    state_store.save(&shared_state);
}
