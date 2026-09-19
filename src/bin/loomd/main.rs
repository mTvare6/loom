// SPDX-License-Identifier: MPL-2.0

mod state;

use directories::ProjectDirs;
use loom_ipc::IpcServer;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::flag;
use state::AudioState;
use std::fs;
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
    const STATE_FILE_NAME: &str = "state.toml";

    fn new() -> Self {
        let path = ProjectDirs::from("com", "epestr", "loom")
            .map(|dirs| dirs.data_dir().join(Self::STATE_FILE_NAME));
        Self { path }
    }

    fn load(&self) -> AudioState {
        let Some(path) = &self.path else {
            return AudioState::new();
        };
        if !path.exists() {
            return AudioState::new();
        }

        let state = || -> Result<AudioState, Box<dyn std::error::Error>> {
            let contents = fs::read_to_string(path)?;
            Ok(toml::from_str(&contents)?)
        };
        match state() {
            Ok(state) => state,
            Err(error) => {
                warn!(
                    "Could not load audio state from {}: {error}",
                    path.display()
                );
                AudioState::new()
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

            let temporary = path.with_extension("toml.tmp");
            fs::write(&temporary, toml::to_string_pretty(state)?)?;
            fs::rename(temporary, path)?;

            Ok(())
        }();

        if let Err(error) = result {
            error!("Could not save audio state to {}: {error}", path.display());
        }
    }
}

struct AudioThread {
    shutdown_tx: loom_pipewire::ShutdownTransmitter,
    thread: Option<JoinHandle<()>>,
}

impl AudioThread {
    fn start(
        state: Arc<AudioState>,
        stop_ipc: Arc<AtomicBool>,
        audio_failed: Arc<AtomicBool>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = loom_pipewire::shutdown_channel();
        let thread = std::thread::spawn(move || {
            let result = loom_pipewire::run_audio_engine(state, shutdown_rx);
            // mainloop stopping without signal handling likely indicates 
            // PipeWire crashing. Handling that separately so init systems can
            // restart (systemd has an option at least)
            if !stop_ipc.load(Ordering::Acquire) {
                match result {
                    Ok(()) => error!("Loom audio engine stopped unexpectedly"),
                    Err(error) => error!("Loom audio stopped: {error}"),
                }
                audio_failed.store(true, Ordering::Release);
                stop_ipc.store(true, Ordering::Release);
            } else if let Err(error) = result {
                error!("Loom audio shutdown failed: {error}");
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

// TODO: Add back logging to file after checking if not running under a init system
fn init_logging() {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_thread_ids(true)
        .init();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    info!("Starting loom daemon");

    let state_store = AudioStateStore::new();
    let shared_state = Arc::new(state_store.load());

    let stop_ipc = Arc::new(AtomicBool::new(false));

    // Sets inner Arc<AtomicBool> to true
    flag::register(SIGINT, stop_ipc.clone()).expect("Couldn't register a SIGINT handler");
    flag::register(SIGTERM, stop_ipc.clone()).expect("Couldn't register a SIGTERM handler");

    let audio_failed = Arc::new(AtomicBool::new(false));
    let audio_thread =
        AudioThread::start(shared_state.clone(), stop_ipc.clone(), audio_failed.clone());

    let ipc_server = IpcServer::new(loom_ipc::socket_path()?);
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
        return Err(io::Error::other("Loom audio engine stopped unexpectedly").into());
    }
    Ok(())
}
