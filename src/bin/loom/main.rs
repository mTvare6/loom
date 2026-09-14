use crate::gui::run_gui;

mod gui;

fn main() -> Result<(), eframe::Error> {
    run_gui("/tmp/loom_audio.sock")
}
