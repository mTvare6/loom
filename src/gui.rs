use crate::state::AudioState;
use eframe::egui;
use std::sync::Arc;

pub fn run_gui(state: Arc<AudioState>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 220.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Loom",
        options,
        Box::new(|_cc| Ok(Box::new(LoomApp { state }))),
    )
}

struct LoomApp {
    state: Arc<AudioState>,
}

impl eframe::App for LoomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Loom");

            let mut current_bypass = self.state.is_bypassed();
            ui.add_space(5.0);
            if ui
                .toggle_value(&mut current_bypass, "Disable")
                .changed()
            {
                self.state.set_bypass(current_bypass);
            }
            ui.separator();
            ui.add_space(10.0);

            let mut current_volume = self.state.volume();
            let mut current_bass = self.state.bass_db();
            let mut current_spatial = self.state.spatial_mix();

            ui.horizontal(|ui| {
                ui.label("Master Volume");
                ui.add(egui::Slider::new(&mut current_volume, 0.0..=2.0));
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Bass in dB");
                ui.add(egui::Slider::new(&mut current_bass, -12.0..=24.0));
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Spatial intensity");
                ui.add(egui::Slider::new(&mut current_spatial, 0.0..=1.0));
            });

            self.state.set_volume(current_volume);
            self.state.set_bass_db(current_bass);
            self.state.set_spatial_mix(current_spatial);
        });

        ctx.request_repaint();
    }
}
