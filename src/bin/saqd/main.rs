// SPDX-License-Identifier: MPL-2.0

mod state;

use directories::ProjectDirs;
use saq_ipc::IpcServer;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::flag;
use state::AudioState;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use tracing::{error, info, warn};

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
    fn start(
        state: Arc<AudioState>,
        stop_ipc: Arc<AtomicBool>,
        audio_failed: Arc<AtomicBool>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = saq_pipewire::shutdown_channel();
        let thread = std::thread::spawn(move || {
            let result = saq_pipewire::run_audio_engine(state, shutdown_rx);
            // mainloop stopping without signal handling likely indicates 
            // PipeWire crashing. Handling that separately so init systems can
            // restart (systemd has an option at least)
            if !stop_ipc.load(Ordering::Acquire) {
                match result {
                    Ok(()) => error!("Śaq audio engine stopped unexpectedly"),
                    Err(error) => error!("Śaq audio stopped: {error}"),
                }
                audio_failed.store(true, Ordering::Release);
                stop_ipc.store(true, Ordering::Release);
            } else if let Err(error) = result {
                error!("Śaq audio shutdown failed: {error}");
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

// TODO: Add back logging to file after checking if not running under a init system
fn init_logging() {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_thread_ids(true)
        .init();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    info!("Starting saq daemon");

    let state_store = AudioStateStore::new();
    let shared_state = Arc::new(state_store.load());

    let stop_ipc = Arc::new(AtomicBool::new(false));

    // Sets inner Arc<AtomicBool> to true
    flag::register(SIGINT, stop_ipc.clone()).expect("Couldn't register a SIGINT handler");
    flag::register(SIGTERM, stop_ipc.clone()).expect("Couldn't register a SIGTERM handler");

    let audio_failed = Arc::new(AtomicBool::new(false));
    let audio_thread =
        AudioThread::start(shared_state.clone(), stop_ipc.clone(), audio_failed.clone());

    let ipc_server = IpcServer::new(saq_ipc::socket_path()?);
    let state = shared_state.clone();

    let ipc_result = ipc_server
        .run_until(Arc::new(move |request| state.handle_query(request)), || {
            // Stops with signals and audio-thread failing
            stop_ipc.load(Ordering::Acquire)
        });
    let audio_failed = audio_failed.load(Ordering::Acquire);

    drop(audio_thread);

    state_store.save(&shared_state);

    ipc_result?;
    if audio_failed {
        return Err(io::Error::other("Śaq audio engine stopped unexpectedly").into());
    }
    Ok(())
}
