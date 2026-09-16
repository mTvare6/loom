use crate::gui::run_gui;

mod gui;

fn main() -> Result<(), eframe::Error> {
    run_gui(saq_ipc::socket_path().expect("Could not resolve the Śaq runtime socket"))
}
