// SPDX-License-Identifier: MPL-2.0

use eframe::egui;
use loom_dsp::Mode;
use loom_ipc::{IpcClient, Request, Response};
use std::path::Path;

// TODO: Improve error output
pub fn run_gui(socket: impl AsRef<Path>) -> eframe::Result<()> {
    let mut ipc_client = match IpcClient::new(socket) {
        Ok(client) => client,
        Err(error) => panic!("{:?}", error),
    };

    let state = ipc_client
        .send(loom_ipc::Request::GetState)
        .expect("Failed to get state");

    let (volume, mode, pitch_enabled, pitch_semitones) = match state {
        Response::State {
            volume,
            mode,
            pitch_enabled,
            pitch,
        } => (volume, Mode::from_u8(mode), pitch_enabled, pitch),
        _ => unreachable!(),
    };

    eframe::run_native(
        "Loom",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([660.0, 310.0])
                .with_resizable(false),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(LoomApp {
                ipc_client,
                volume,
                mode,
                pitch_enabled,
                pitch_semitones,
            }))
        }),
    )
}

struct LoomApp {
    ipc_client: IpcClient,
    volume: f32,
    mode: Mode,
    pitch_enabled: bool,
    pitch_semitones: f32,
}

impl eframe::App for LoomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Loom");
            });
            ui.add_space(10.0);

            let available_width = ui.available_width();
            let volume_width = available_width * 0.75;
            let mut volume_updated = false;
            ui.horizontal(|ui| {
                ui.add_space((available_width - volume_width) * 0.5);
                ui.spacing_mut().slider_width = volume_width;
                volume_updated = ui
                    .add(egui::Slider::new(&mut self.volume, 0.0..=2.0).show_value(false))
                    .changed();
            });

            ui.add_space(12.0);
            let (surround, spatial_filter, spatial_stereo, spatial_surround) = ui
                .horizontal(|ui| {
                    const BUTTON_ROW_WIDTH: f32 = 360.0;
                    ui.add_space(((ui.available_width() - BUTTON_ROW_WIDTH) * 0.5).max(0.0));
                    let surround = mode_button(
                        ui,
                        "3D Surround",
                        ModeIcon::Cube,
                        self.mode == Mode::Surround3d,
                    );
                    let spatial_filter = mode_button(
                        ui,
                        "Spatial Filter",
                        ModeIcon::Spatial,
                        self.mode == Mode::SpatialFilter,
                    );
                    let spatial_stereo = mode_button(
                        ui,
                        "Spatial Stereo",
                        ModeIcon::Spatial,
                        self.mode == Mode::SpatialStereo,
                    );
                    let spatial_surround = mode_button(
                        ui,
                        "Spatial Surround",
                        ModeIcon::Spatial,
                        self.mode == Mode::SpatialSurround,
                    );
                    (surround, spatial_filter, spatial_stereo, spatial_surround)
                })
                .inner;

            let mut pitch_updated = false;
            ui.add_space(4.0);
            let (ambience, fidelity, night) = ui
                .horizontal(|ui| {
                    const BUTTON_ROW_WIDTH: f32 = 360.0;
                    ui.add_space(((ui.available_width() - BUTTON_ROW_WIDTH) * 0.5).max(0.0));
                    let ambience = mode_button(
                        ui,
                        "Ambience",
                        ModeIcon::Ambience,
                        self.mode == Mode::Ambience,
                    );
                    let fidelity = mode_button(
                        ui,
                        "Fidelity",
                        ModeIcon::Fidelity,
                        self.mode == Mode::Fidelity,
                    );
                    let night = mode_button(ui, "Night", ModeIcon::Night, self.mode == Mode::Night);
                    pitch_updated =
                        pitch_control(ui, &mut self.pitch_enabled, &mut self.pitch_semitones)
                            .changed();
                    (ambience, fidelity, night)
                })
                .inner;

            let mut mode_updated = true;
            self.mode = if surround.clicked() {
                toggle(self.mode, Mode::Surround3d)
            } else if spatial_filter.clicked() {
                toggle(self.mode, Mode::SpatialFilter)
            } else if spatial_stereo.clicked() {
                toggle(self.mode, Mode::SpatialStereo)
            } else if spatial_surround.clicked() {
                toggle(self.mode, Mode::SpatialSurround)
            } else if ambience.clicked() {
                toggle(self.mode, Mode::Ambience)
            } else if fidelity.clicked() {
                toggle(self.mode, Mode::Fidelity)
            } else if night.clicked() {
                toggle(self.mode, Mode::Night)
            } else {
                mode_updated = false;
                self.mode
            };

            if mode_updated {
                let _ = self.ipc_client.send(Request::SetMode(self.mode as u8));
            }
            if pitch_updated {
                let _ = self
                    .ipc_client
                    .send(Request::SetPitchEnabled(self.pitch_enabled));
                let _ = self
                    .ipc_client
                    .send(Request::SetPitch(self.pitch_semitones));
            }
            if volume_updated {
                let _ = self.ipc_client.send(Request::SetVolume(self.volume));
            }
        });

        ctx.request_repaint();
    }
}

#[derive(Clone, Copy)]
enum ModeIcon {
    Spatial,
    Cube,
    Ambience,
    Fidelity,
    Night,
}

fn toggle(selected: Mode, clicked: Mode) -> Mode {
    if selected == clicked {
        Mode::Off
    } else {
        clicked
    }
}

fn mode_button(ui: &mut egui::Ui, label: &str, icon: ModeIcon, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(84.0, 80.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let visuals = ui.style().interact_selectable(&response, selected);
    let icon_center = egui::pos2(rect.center().x, rect.top() + 28.0);
    if selected {
        ui.painter().circle_filled(
            icon_center,
            19.0,
            egui::Color32::from_rgba_unmultiplied(57, 143, 255, 12),
        );
        ui.painter().circle_filled(
            icon_center,
            14.0,
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
        ModeIcon::Ambience => paint_ambience(ui.painter(), icon_center, icon_color),
        ModeIcon::Fidelity => paint_fidelity(ui.painter(), icon_center, icon_color),
        ModeIcon::Night => paint_night(ui.painter(), icon_center, icon_color),
    }

    let text_position = egui::pos2(rect.center().x, rect.bottom() - 13.0);
    let font = egui::FontId::proportional(11.0);
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

fn paint_ambience(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    painter.circle_filled(center, 2.5, color);
    for radius in [8.0_f32, 15.0] {
        let points = (0..=16)
            .map(|index| {
                let angle = -1.25_f32 + 2.5 * index as f32 / 16.0;
                center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
            })
            .collect();
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.4_f32, color)));
    }
}

fn paint_fidelity(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let points = [
        center + egui::vec2(-17.0, 4.0),
        center + egui::vec2(-11.0, -5.0),
        center + egui::vec2(-4.0, 8.0),
        center + egui::vec2(4.0, -9.0),
        center + egui::vec2(11.0, 4.0),
        center + egui::vec2(17.0, -2.0),
    ];
    painter.add(egui::Shape::line(points.to_vec(), stroke));
}

fn paint_night(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let outer: Vec<_> = (0..=20)
        .map(|index| {
            let angle = 0.65_f32 + 4.9 * index as f32 / 20.0;
            center + egui::vec2(angle.cos() * 15.0, angle.sin() * 15.0)
        })
        .collect();
    let inner: Vec<_> = (0..=20)
        .map(|index| {
            let angle = 0.85_f32 + 4.5 * (20 - index) as f32 / 20.0;
            center + egui::vec2(6.0 + angle.cos() * 11.0, angle.sin() * 11.0)
        })
        .collect();
    painter.add(egui::Shape::line(
        outer.into_iter().chain(inner).collect(),
        egui::Stroke::new(1.7_f32, color),
    ));
}

fn pitch_control(ui: &mut egui::Ui, enabled: &mut bool, semitones: &mut f32) -> egui::Response {
    use std::f32::consts::{FRAC_PI_2, PI};

    const SIZE: egui::Vec2 = egui::vec2(84.0, 90.0);
    const ARC_RADIUS: f32 = 26.0;
    const ARC_HALF_ANGLE: f32 = PI * 0.75;
    const INNER_RADIUS: f32 = 16.0;

    let (rect, response) = ui.allocate_exact_size(SIZE, egui::Sense::click_and_drag());
    let mut response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = egui::pos2(rect.center().x, rect.top() + 38.0);
    let pointer = response.interact_pointer_pos();
    let pointer_radius = pointer.map_or(0.0, |position| position.distance(center));
    let was_enabled = *enabled;

    if response.clicked() {
        if !was_enabled {
            *enabled = true;
            response.mark_changed();
        } else if pointer_radius <= INNER_RADIUS {
            *enabled = false;
            response.mark_changed();
        }
    }

    if was_enabled
        && *enabled
        && pointer_radius > INNER_RADIUS
        && (response.dragged() || response.clicked())
        && let Some(pointer) = pointer
    {
        *semitones = pitch_from_direction(pointer - center);
        response.mark_changed();
    }

    let visuals = ui.style().interact_selectable(&response, *enabled);
    let icon_color = if *enabled {
        egui::Color32::from_rgb(91, 182, 255)
    } else if response.hovered() {
        egui::Color32::from_rgb(220, 225, 235)
    } else {
        visuals.fg_stroke.color
    };

    if *enabled {
        ui.painter().circle_filled(
            center,
            16.0,
            egui::Color32::from_rgba_unmultiplied(57, 143, 255, 18),
        );
        let arc = (0..=48)
            .map(|index| {
                let offset = -ARC_HALF_ANGLE + 2.0 * ARC_HALF_ANGLE * index as f32 / 48.0;
                let angle = offset - FRAC_PI_2;
                center + egui::vec2(angle.cos(), angle.sin()) * ARC_RADIUS
            })
            .collect();
        ui.painter().add(egui::Shape::line(
            arc,
            egui::Stroke::new(1.5_f32, egui::Color32::from_gray(85)),
        ));

        let zero_inner = center + egui::vec2(0.0, -ARC_RADIUS + 3.0);
        let zero_outer = center + egui::vec2(0.0, -ARC_RADIUS - 4.0);
        ui.painter().line_segment(
            [zero_inner, zero_outer],
            egui::Stroke::new(1.7_f32, egui::Color32::from_gray(190)),
        );
        ui.painter().text(
            center + egui::vec2(0.0, -ARC_RADIUS - 5.0),
            egui::Align2::CENTER_BOTTOM,
            ((*semitones * 10.).round() / 10.).to_string(),
            egui::FontId::proportional(9.0),
            egui::Color32::from_gray(190),
        );

        let marker_offset = semitones.clamp(-12.0, 12.0) / 12.0 * ARC_HALF_ANGLE;
        let marker_angle = marker_offset - FRAC_PI_2;
        let marker = center + egui::vec2(marker_angle.cos(), marker_angle.sin()) * ARC_RADIUS;
        ui.painter().circle_filled(marker, 3.4, icon_color);
    }

    paint_pitch(ui.painter(), center, icon_color);
    let text_position = egui::pos2(rect.center().x, rect.bottom() - 5.0);
    let font = egui::FontId::proportional(11.0);
    ui.painter().text(
        text_position,
        egui::Align2::CENTER_BOTTOM,
        "Pitch",
        font.clone(),
        visuals.text_color(),
    );
    if *enabled {
        ui.painter().text(
            text_position + egui::vec2(0.45, 0.0),
            egui::Align2::CENTER_BOTTOM,
            "Pitch",
            font,
            visuals.text_color(),
        );
    }

    response
}

fn pitch_from_direction(direction: egui::Vec2) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI};

    const ARC_HALF_ANGLE: f32 = PI * 0.75;
    const ZERO_SNAP_SEMITONES: f32 = 0.25;
    let mut angle_from_zero = direction.y.atan2(direction.x) + FRAC_PI_2;
    if angle_from_zero > PI {
        angle_from_zero -= 2.0 * PI;
    } else if angle_from_zero < -PI {
        angle_from_zero += 2.0 * PI;
    }
    let raw = angle_from_zero.clamp(-ARC_HALF_ANGLE, ARC_HALF_ANGLE) / ARC_HALF_ANGLE * 12.0;
    if raw.abs() <= ZERO_SNAP_SEMITONES {
        0.0
    } else {
        raw
    }
}

fn paint_pitch(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6_f32, color);
    painter.line_segment(
        [
            center + egui::vec2(-7.0, -8.0),
            center + egui::vec2(-7.0, 8.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(-12.0, 3.0),
            center + egui::vec2(-7.0, 8.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(-2.0, 3.0),
            center + egui::vec2(-7.0, 8.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(7.0, -8.0),
            center + egui::vec2(7.0, 8.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(2.0, -3.0),
            center + egui::vec2(7.0, -8.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(12.0, -3.0),
            center + egui::vec2(7.0, -8.0),
        ],
        stroke,
    );
}
