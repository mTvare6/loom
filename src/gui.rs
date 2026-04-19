use crate::state::AudioState;
use eframe::egui;
use std::sync::Arc;

pub fn run_gui(state: Arc<AudioState>) -> eframe::Result<()> {
    eframe::run_native(
        "Loom",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([380.0, 180.0])
                .with_resizable(false),
            ..Default::default()
        },
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
            if ui.toggle_value(&mut current_bypass, "Disable").changed() {
                self.state.set_bypass(current_bypass);
            }
            ui.separator();
            ui.add_space(10.0);

            let mut current_volume = self.state.volume();
            let mut current_spatial = self.state.spatial_mix();

            ui.horizontal(|ui| {
                ui.label("Master Volume");
                ui.add(egui::Slider::new(&mut current_volume, 0.0..=2.0));
            });

            ui.add_space(10.0);
            ui.add_enabled_ui(!current_bypass, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Spatial intensity");
                    ui.add(egui::Slider::new(&mut current_spatial, 0.0..=1.0));
                });
            });

            self.state.set_volume(current_volume);
            self.state.set_spatial_mix(current_spatial);
        });

        ctx.request_repaint();
    }
}
