mod audio;
mod dsp;
mod gui;
mod state;

use state::AudioState;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let shared_state = Arc::new(AudioState::new(1.0));

    let audio_state = shared_state.clone();
    std::thread::spawn(move || {
        audio::run_audio_engine(audio_state).expect("Loom crashed");
    });

    gui::run_gui(shared_state)
}
