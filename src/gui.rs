use crate::state::AudioState;
use eframe::egui;
use saq_dsp::Mode;
use std::sync::Arc;

pub fn run_gui(state: Arc<AudioState>) -> eframe::Result<()> {
    eframe::run_native(
        "Śaq",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([420.0, 220.0])
                .with_resizable(false),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(SaqApp { state }))
        }),
    )
}

struct SaqApp {
    state: Arc<AudioState>,
}

impl eframe::App for SaqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Śaq");
            });
            ui.add_space(10.0);

            let mut volume = self.state.volume();
            let available_width = ui.available_width();
            let volume_width = available_width * 0.75;
            ui.horizontal(|ui| {
                ui.add_space((available_width - volume_width) * 0.5);
                ui.spacing_mut().slider_width = volume_width;
                ui.add(egui::Slider::new(&mut volume, 0.0..=2.0).show_value(false));
            });
            self.state.set_volume(volume);

            ui.add_space(12.0);
            let selected = self.state.mode();
            ui.horizontal(|ui| {
                const BUTTON_ROW_WIDTH: f32 = 232.0;
                ui.add_space(((ui.available_width() - BUTTON_ROW_WIDTH) * 0.5).max(0.0));
                let surround = mode_button(
                    ui,
                    "3D Surround",
                    ModeIcon::Cube,
                    selected == Mode::SurroundSound,
                );
                let spatial =
                    mode_button(ui, "Spatial", ModeIcon::Spatial, selected == Mode::Spatial);

                if surround.clicked() {
                    self.state.set_mode(toggle(selected, Mode::SurroundSound));
                } else if spatial.clicked() {
                    self.state.set_mode(toggle(selected, Mode::Spatial));
                }
            });
        });

        ctx.request_repaint();
    }
}

#[derive(Clone, Copy)]
enum ModeIcon {
    Spatial,
    Cube,
}

fn toggle(selected: Mode, clicked: Mode) -> Mode {
    if selected == clicked {
        Mode::Off
    } else {
        clicked
    }
}

fn mode_button(ui: &mut egui::Ui, label: &str, icon: ModeIcon, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(112.0, 112.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let visuals = ui.style().interact_selectable(&response, selected);
    let icon_center = egui::pos2(rect.center().x, rect.top() + 42.0);
    if selected {
        ui.painter().circle_filled(
            icon_center,
            28.0,
            egui::Color32::from_rgba_unmultiplied(57, 143, 255, 12),
        );
        ui.painter().circle_filled(
            icon_center,
            21.0,
            egui::Color32::from_rgba_unmultiplied(57, 143, 255, 20),
        );
    }

    let icon_color = if selected {
        egui::Color32::from_rgb(91, 182, 255)
    } else if response.hovered() {
        egui::Color32::from_rgb(220, 225, 235)
    } else {
        visuals.fg_stroke.color
    };
    match icon {
        ModeIcon::Spatial => paint_spatial(ui.painter(), icon_center, icon_color),
        ModeIcon::Cube => paint_cube(ui.painter(), icon_center, icon_color),
    }

    let text_position = egui::pos2(rect.center().x, rect.bottom() - 13.0);
    let font = egui::FontId::proportional(14.0);
    ui.painter().text(
        text_position,
        egui::Align2::CENTER_BOTTOM,
        label,
        font.clone(),
        visuals.text_color(),
    );
    if selected {
        ui.painter().text(
            text_position + egui::vec2(0.45, 0.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            font,
            visuals.text_color(),
        );
    }

    response
}

fn paint_spatial(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    painter.circle_filled(center, 3.0, color);
    painter.circle_stroke(center, 10.0, egui::Stroke::new(1.5_f32, color));
    painter.circle_stroke(center, 18.0, egui::Stroke::new(1.5_f32, color));
}

fn paint_cube(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let top = center + egui::vec2(0.0, -17.0);
    let left = center + egui::vec2(-16.0, -8.0);
    let right = center + egui::vec2(16.0, -8.0);
    let middle = center + egui::vec2(0.0, 1.0);
    let bottom_left = center + egui::vec2(-16.0, 10.0);
    let bottom_right = center + egui::vec2(16.0, 10.0);
    let bottom = center + egui::vec2(0.0, 19.0);

    for [from, to] in [
        [top, left],
        [top, right],
        [left, middle],
        [right, middle],
        [left, bottom_left],
        [right, bottom_right],
        [middle, bottom],
        [bottom_left, bottom],
        [bottom_right, bottom],
    ] {
        painter.line_segment([from, to], stroke);
    }
}
