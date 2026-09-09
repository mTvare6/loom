mod gui;
mod state;

use loom_ipc::IpcServer;
use state::AudioState;
use std::{env::args, sync::Arc, thread::JoinHandle};

struct AudioThread {
    shutdown_tx: loom_pipewire::ShutdownTransmitter,
    thread: Option<JoinHandle<()>>,
}

impl AudioThread {
    fn start(state: Arc<AudioState>) -> Self {
        let (shutdown_tx, shutdown_rx) = loom_pipewire::shutdown_channel();
        let thread = std::thread::spawn(move || {
            if let Err(error) = loom_pipewire::run_audio_engine(state, shutdown_rx) {
                eprintln!("Loom audio stopped: {error}");
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
            eprintln!("Loom audio thread panicked");
        }
    }
}

fn main() -> eframe::Result<()> {
    let shared_state = Arc::new(AudioState::new(1.0));
    let _audio_thread = AudioThread::start(shared_state.clone());

    let args: Vec<String> = args().collect();

    if args.contains(&"--headless".to_string()) {
        let ipc_server = IpcServer::new("/tmp/loom_audio.sock");
        let state = shared_state.clone();
        ipc_server.run(Arc::new(move |request| state.handle_query(request)));
        Ok(())
    } else if args.contains(&"--gui".to_string()) {
        gui::run_gui(shared_state)
    } else {
        panic!("Use '--headless' or '--gui'")
    }
}
