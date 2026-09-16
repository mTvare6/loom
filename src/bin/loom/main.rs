use crate::gui::run_gui;

mod gui;

fn main() -> Result<(), eframe::Error> {
    run_gui(loom_ipc::socket_path().expect("Could not resolve the Loom runtime socket"))
}
