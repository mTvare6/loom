// SPDX-License-Identifier: MPL-2.0

use eframe::egui;
use saq_dsp::{EQ_BAND_COUNT, EQ_BAND_FREQUENCIES, EQ_MAX_POINTS, EqPreset, EqProfile, Mode};
use saq_ipc::{Event, IpcClient, Request, Response};
use palette::{Clamp, FromColor, Mix, Oklab, Srgb};
use std::path::Path;
use std::time::Duration;

const DARK0_HARD: egui::Color32 = egui::Color32::from_rgb(29, 32, 33);
const DARK0: egui::Color32 = egui::Color32::from_rgb(40, 40, 40);
const DARK0_SOFT: egui::Color32 = egui::Color32::from_rgb(50, 48, 47);
const DARK2: egui::Color32 = egui::Color32::from_rgb(80, 73, 69);
const LIGHT0: egui::Color32 = egui::Color32::from_rgb(251, 241, 199);
const LIGHT1: egui::Color32 = egui::Color32::from_rgb(235, 219, 178);
const LIGHT3: egui::Color32 = egui::Color32::from_rgb(189, 174, 147);
const BRIGHT_RED: egui::Color32 = egui::Color32::from_rgb(251, 73, 52);
const BRIGHT_GREEN: egui::Color32 = egui::Color32::from_rgb(184, 187, 38);
const BRIGHT_YELLOW: egui::Color32 = egui::Color32::from_rgb(250, 189, 47);
const BRIGHT_BLUE: egui::Color32 = egui::Color32::from_rgb(131, 165, 152);
const BRIGHT_PURPLE: egui::Color32 = egui::Color32::from_rgb(211, 134, 155);
const BRIGHT_AQUA: egui::Color32 = egui::Color32::from_rgb(142, 192, 124);
const BRIGHT_ORANGE: egui::Color32 = egui::Color32::from_rgb(254, 128, 25);
const LOGO_BLACK: egui::Color32 = egui::Color32::from_rgb(40, 24, 18);

const CONTENT_WIDTH: f32 = 672.0;
const CARD_SIZE: egui::Vec2 = egui::vec2(162.0, 88.0);
const WINDOW_SIZE: egui::Vec2 = egui::vec2(708.0, 580.0);
const EQ_GRAPH_HEIGHT: f32 = 202.0;

pub fn run_gui(socket: impl AsRef<Path>) -> eframe::Result<()> {
    let mut ipc_client = match IpcClient::new(socket) {
        Ok(client) => client,
        Err(error) => panic!("{:?}", error),
    };

    let state = ipc_client
        .send(Request::GetState)
        .expect("Failed to get state");

    let (
        volume,
        mode,
        pitch_enabled,
        pitch_semitones,
        subwoofer,
        eq_preset,
        eq_base_preset,
        eq_point_count,
        eq_frequencies_hz,
        eq_gains_db,
    ) = match state {
        Response::State {
            volume,
            mode,
            pitch_enabled,
            pitch,
            subwoofer,
            eq_preset,
            eq_base_preset,
            eq_point_count,
            eq_frequencies_hz,
            eq_gains_db,
        } => (
            volume,
            Mode::from_u8(mode),
            pitch_enabled,
            pitch,
            subwoofer,
            EqPreset::from_u8(eq_preset),
            EqPreset::from_u8(eq_base_preset),
            eq_point_count,
            array_from_vec(eq_frequencies_hz, EqProfile::default_frequencies()),
            array_from_vec(eq_gains_db, [0.0; EQ_MAX_POINTS]),
        ),
        _ => unreachable!(),
    };

    eframe::run_native(
        "Śaq",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(WINDOW_SIZE)
                .with_min_inner_size(WINDOW_SIZE)
                .with_resizable(false),
            persist_window: false,
            ..Default::default()
        },
        Box::new(|cc| {
            configure_style(&cc.egui_ctx);
            Ok(Box::new(SaqApp {
                ipc_client,
                volume,
                mode,
                pitch_enabled,
                pitch_semitones,
                subwoofer,
                eq_preset,
                eq_base_preset,
                eq_point_count,
                eq_frequencies_hz,
                eq_gains_db,
                vocals_response: EqPreset::Dialogue
                    .response_db(&EQ_BAND_FREQUENCIES)
                    .try_into()
                    .expect("31 EQ response points"),
                active_eq_handle: None,
                window_size_initialized: false,
            }))
        }),
    )
}

fn handle_response(response: std::io::Result<Response>) {
    match response {
        Err(err) => {
            eprintln!("Śaq daemon error: {}", err)
        }
        Ok(Response::Error(error)) => {
            eprintln!("Error: {}", error);
        }
        _ => {}
    }
}

fn array_from_vec<T: Copy, const N: usize>(values: Vec<T>, mut defaults: [T; N]) -> [T; N] {
    for (target, value) in defaults.iter_mut().zip(values) {
        *target = value;
    }
    defaults
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(LIGHT1);
    style.visuals.panel_fill = DARK0;
    style.visuals.window_fill = DARK0;
    style.visuals.extreme_bg_color = DARK0_HARD;
    style.visuals.faint_bg_color = DARK0_SOFT;
    style.visuals.selection.bg_fill = DARK2;
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, LIGHT0);
    style.spacing.item_spacing = egui::vec2(8.0, 2.0);
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(10.0, egui::FontFamily::Monospace),
    );
    ctx.set_style(style);
}

struct SaqApp {
    ipc_client: IpcClient,
    volume: f32,
    mode: Mode,
    pitch_enabled: bool,
    pitch_semitones: f32,
    subwoofer: f32,
    eq_preset: EqPreset,
    eq_base_preset: EqPreset,
    eq_point_count: u8,
    eq_frequencies_hz: [f32; EQ_MAX_POINTS],
    eq_gains_db: [f32; EQ_MAX_POINTS],
    vocals_response: [f32; EQ_BAND_COUNT],
    active_eq_handle: Option<usize>,
    window_size_initialized: bool,
}

impl SaqApp {
    // poll for events
    fn poll_ipc_events(&mut self, ctx: &egui::Context) {
        if let Ok(event) = self.ipc_client.try_recv_event(Duration::from_millis(1)) {
            match event {
                Event::StateUpdated {
                    volume,
                    mode,
                    pitch_enabled,
                    pitch,
                    subwoofer,
                    eq_preset,
                    eq_base_preset,
                    eq_point_count,
                    eq_frequencies_hz,
                    eq_gains_db,
                } => {
                    self.volume = volume;
                    self.mode = Mode::from_u8(mode);
                    self.pitch_enabled = pitch_enabled;
                    self.pitch_semitones = pitch;
                    self.subwoofer = subwoofer;
                    self.eq_preset = EqPreset::from_u8(eq_preset);
                    self.eq_base_preset = EqPreset::from_u8(eq_base_preset);
                    self.eq_point_count = eq_point_count;
                    self.eq_frequencies_hz =
                        array_from_vec(eq_frequencies_hz, EqProfile::default_frequencies());
                    self.eq_gains_db = array_from_vec(eq_gains_db, [0.0; EQ_MAX_POINTS]);
                    ctx.request_repaint();
                }
            }
        }
    }
}

impl eframe::App for SaqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.window_size_initialized {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(WINDOW_SIZE));
            self.window_size_initialized = true;
        }

        self.poll_ipc_events(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(ui.max_rect(), 0, DARK0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(CONTENT_WIDTH);
                title_bar(ui);

                let volume = volume_control(ui, &mut self.volume);
                let eq_changed = eq_control(
                    ui,
                    EqControl {
                        preset: &mut self.eq_preset,
                        base_preset: &mut self.eq_base_preset,
                        point_count: &mut self.eq_point_count,
                        frequencies_hz: &mut self.eq_frequencies_hz,
                        gains_db: &mut self.eq_gains_db,
                        active_handle: &mut self.active_eq_handle,
                    },
                    &self.vocals_response,
                );

                let (
                    (surround_sound_clicked, subwoofer_changed),
                    spatial_filter,
                    spatial_stereo,
                    spatial_surround,
                ) = ui
                    .horizontal(|ui| {
                        (
                            surround_sound_control(
                                ui,
                                self.mode == Mode::SurroundSound,
                                &mut self.subwoofer,
                            ),
                            mode_button(
                                ui,
                                "SPATIAL FILTER",
                                ModeIcon::Spatial,
                                BRIGHT_AQUA,
                                self.mode == Mode::SpatialFilter,
                            ),
                            mode_button(
                                ui,
                                "SPATIAL STEREO",
                                ModeIcon::Stereo,
                                BRIGHT_BLUE,
                                self.mode == Mode::SpatialStereo,
                            ),
                            mode_button(
                                ui,
                                "SPATIAL SURROUND",
                                ModeIcon::Orbit,
                                BRIGHT_PURPLE,
                                self.mode == Mode::SpatialSurround,
                            ),
                        )
                    })
                    .inner;

                let (room, clarity, night, pitch) = ui
                    .horizontal(|ui| {
                        (
                            mode_button(
                                ui,
                                "ROOM",
                                ModeIcon::Room,
                                BRIGHT_GREEN,
                                self.mode == Mode::Room,
                            ),
                            mode_button(
                                ui,
                                "CLARITY",
                                ModeIcon::Clarity,
                                BRIGHT_YELLOW,
                                self.mode == Mode::Clarity,
                            ),
                            mode_button(
                                ui,
                                "NIGHT",
                                ModeIcon::Night,
                                BRIGHT_RED,
                                self.mode == Mode::Night,
                            ),
                            pitch_control(ui, &mut self.pitch_enabled, &mut self.pitch_semitones),
                        )
                    })
                    .inner;

                let new_mode = if surround_sound_clicked {
                    toggle(self.mode, Mode::SurroundSound)
                } else if spatial_filter.clicked() {
                    toggle(self.mode, Mode::SpatialFilter)
                } else if spatial_stereo.clicked() {
                    toggle(self.mode, Mode::SpatialStereo)
                } else if spatial_surround.clicked() {
                    toggle(self.mode, Mode::SpatialSurround)
                } else if room.clicked() {
                    toggle(self.mode, Mode::Room)
                } else if clarity.clicked() {
                    toggle(self.mode, Mode::Clarity)
                } else if night.clicked() {
                    toggle(self.mode, Mode::Night)
                } else {
                    self.mode
                };

                if new_mode != self.mode {
                    handle_response(self.ipc_client.send(Request::SetMode(new_mode as u8)));
                    self.mode = new_mode;
                }
                if subwoofer_changed {
                    handle_response(self.ipc_client.send(Request::SetSubwoofer(self.subwoofer)));
                }
                if volume.changed() {
                    handle_response(self.ipc_client.send(Request::SetVolume(self.volume)));
                }
                if eq_changed {
                    handle_response(self.ipc_client.send(Request::SetEqProfile {
                        preset: self.eq_preset as u8,
                        base_preset: self.eq_base_preset as u8,
                        point_count: self.eq_point_count,
                        frequencies_hz: self.eq_frequencies_hz.to_vec(),
                        gains_db: self.eq_gains_db.to_vec(),
                    }));
                }
                if pitch.changed() {
                    handle_response(
                        self.ipc_client
                            .send(Request::SetPitchEnabled(self.pitch_enabled)),
                    );
                    handle_response(
                        self.ipc_client
                            .send(Request::SetPitch(self.pitch_semitones)),
                    );
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

struct EqControl<'a> {
    preset: &'a mut EqPreset,
    base_preset: &'a mut EqPreset,
    point_count: &'a mut u8,
    frequencies_hz: &'a mut [f32; EQ_MAX_POINTS],
    gains_db: &'a mut [f32; EQ_MAX_POINTS],
    active_handle: &'a mut Option<usize>,
}

fn eq_control(
    ui: &mut egui::Ui,
    control: EqControl<'_>,
    vocals_response: &[f32; EQ_BAND_COUNT],
) -> bool {
    let EqControl {
        preset,
        base_preset,
        point_count,
        frequencies_hz,
        gains_db,
        active_handle,
    } = control;
    let mut committed = false;
    let previous_preset = *preset;
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("EQUALIZER")
                .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                .color(LIGHT3),
        );
        egui::ComboBox::from_id_salt("eq-preset")
            .selected_text(preset.label())
            .width(112.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(preset, EqPreset::Off, EqPreset::Off.label());
                ui.selectable_value(preset, EqPreset::Dialogue, EqPreset::Dialogue.label());
            });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            if ui.small_button("RESET").clicked() {
                gains_db.fill(0.0);
                *frequencies_hz = EqProfile::default_frequencies();
                *point_count = EQ_BAND_COUNT as u8;
                *preset = *base_preset;
                *active_handle = None;
                committed = true;
            }
        });
    });

    if *preset != previous_preset {
        *base_preset = *preset;
        gains_db.fill(0.0);
        *frequencies_hz = EqProfile::default_frequencies();
        *point_count = EQ_BAND_COUNT as u8;
        *active_handle = None;
        committed = true;
    }
    *point_count = (*point_count).clamp(1, EQ_MAX_POINTS as u8);
    let model_points = *point_count as usize;

    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(CONTENT_WIDTH, EQ_GRAPH_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    let response = response.on_hover_cursor(egui::CursorIcon::Crosshair);
    let graph = egui::Rect::from_min_max(
        rect.min + egui::vec2(42.0, 10.0),
        rect.max - egui::vec2(12.0, 26.0),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect.shrink(2.0), 4.0, DARK0_HARD);

    const MIN_DB: f32 = -36.0;
    const MAX_DB: f32 = 18.0;
    let x_for_frequency = |frequency: f32| {
        let amount = (frequency / 20.0).ln() / (20_000.0_f32 / 20.0).ln();
        egui::lerp(graph.x_range(), amount.clamp(0.0, 1.0))
    };
    let frequency_for_x = |x: f32| {
        let amount = ((x - graph.left()) / graph.width()).clamp(0.0, 1.0);
        20.0 * (20_000.0_f32 / 20.0).powf(amount)
    };
    let y_for_db = |db: f32| {
        egui::lerp(
            graph.y_range(),
            ((MAX_DB - db) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0),
        )
    };
    let db_for_y =
        |y: f32| MAX_DB - ((y - graph.top()) / graph.height()).clamp(0.0, 1.0) * (MAX_DB - MIN_DB);

    for db in [-30.0, -18.0, -6.0, 0.0, 6.0, 12.0] {
        let y = y_for_db(db);
        let color = if db == 0.0 { DARK2 } else { DARK0_SOFT };
        painter.line_segment(
            [egui::pos2(graph.left(), y), egui::pos2(graph.right(), y)],
            egui::Stroke::new(1.0_f32, color),
        );
        painter.text(
            egui::pos2(graph.left() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{db:+.0}"),
            egui::FontId::new(8.0, egui::FontFamily::Monospace),
            LIGHT3,
        );
    }

    for (index, frequency) in frequencies_hz
        .iter()
        .copied()
        .take(model_points)
        .enumerate()
    {
        let x = x_for_frequency(frequency);
        painter.line_segment(
            [egui::pos2(x, graph.top()), egui::pos2(x, graph.bottom())],
            egui::Stroke::new(0.5_f32, DARK0_SOFT),
        );
        let show_label = index == 0 || index + 1 == model_points || index % 3 == 1;
        if show_label {
            painter.text(
                egui::pos2(x, graph.bottom() + 9.0),
                egui::Align2::CENTER_CENTER,
                frequency_label(frequency),
                egui::FontId::new(8.0, egui::FontFamily::Monospace),
                LIGHT3,
            );
        }
    }

    let displayed_base = if *preset == EqPreset::Custom {
        *base_preset
    } else {
        *preset
    };
    let custom_displayed = *preset == EqPreset::Custom;
    let displayed_frequencies = *frequencies_hz;
    let displayed_point_count = model_points;
    let base_at = |frequency: f32| {
        if displayed_base == EqPreset::Dialogue {
            sampled_values(frequency, &EQ_BAND_FREQUENCIES, vocals_response)
        } else {
            0.0
        }
    };
    let gain_at = |frequency: f32, gains: &[f32; EQ_MAX_POINTS]| {
        if custom_displayed {
            sampled_values(
                frequency,
                &displayed_frequencies[..displayed_point_count],
                &gains[..displayed_point_count],
            )
        } else {
            0.0
        }
    };

    let mut curve = Vec::with_capacity(161);
    for point in 0..=160 {
        let amount = point as f32 / 160.0;
        let frequency = 20.0 * (20_000.0_f32 / 20.0).powf(amount);
        let db = base_at(frequency) + gain_at(frequency, gains_db);
        curve.push(egui::pos2(x_for_frequency(frequency), y_for_db(db)));
    }
    let mut fill = curve.clone();
    fill.push(egui::pos2(graph.right(), y_for_db(MIN_DB)));
    fill.push(egui::pos2(graph.left(), y_for_db(MIN_DB)));
    painter.add(egui::Shape::convex_polygon(
        fill,
        BRIGHT_BLUE.gamma_multiply(0.10),
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::line(
        curve,
        egui::Stroke::new(2.0_f32, BRIGHT_BLUE),
    ));

    if (response.drag_started() || response.clicked())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        *active_handle = None;
        let nearest = (0..model_points).min_by(|left, right| {
            let left_distance = (x_for_frequency(frequencies_hz[*left]) - pointer.x).abs();
            let right_distance = (x_for_frequency(frequencies_hz[*right]) - pointer.x).abs();
            left_distance.total_cmp(&right_distance)
        });
        let existing = nearest
            .filter(|index| (x_for_frequency(frequencies_hz[*index]) - pointer.x).abs() <= 8.0);
        if let Some(index) = existing {
            *active_handle = Some(index);
        } else if model_points < EQ_MAX_POINTS {
            if *preset != EqPreset::Custom {
                *base_preset = *preset;
                gains_db.fill(0.0);
                *preset = EqPreset::Custom;
            }
            let frequency = frequency_for_x(pointer.x);
            *active_handle = insert_eq_point(
                point_count,
                frequencies_hz,
                gains_db,
                frequency,
                db_for_y(pointer.y) - base_at(frequency),
            );
        }
    }
    if (response.dragged() || response.clicked())
        && let (Some(index), Some(pointer)) = (*active_handle, response.interact_pointer_pos())
    {
        if *preset != EqPreset::Custom {
            *base_preset = *preset;
            gains_db.fill(0.0);
            *preset = EqPreset::Custom;
        }
        let target = db_for_y(pointer.y);
        gains_db[index] = (target - base_at(frequencies_hz[index])).clamp(-12.0, 12.0);
        if response.clicked() {
            committed = true;
            *active_handle = None;
        }
    }
    if response.drag_stopped() {
        committed = active_handle.is_some();
        *active_handle = None;
    }

    for (index, frequency) in frequencies_hz
        .iter()
        .copied()
        .take(*point_count as usize)
        .enumerate()
    {
        let db = base_at(frequency) + gain_at(frequency, gains_db);
        let center = egui::pos2(x_for_frequency(frequency), y_for_db(db));
        let selected = *active_handle == Some(index);
        painter.circle_filled(center, if selected { 5.0 } else { 3.5 }, DARK0_HARD);
        painter.circle_stroke(
            center,
            if selected { 5.0 } else { 3.5 },
            egui::Stroke::new(1.5_f32, if selected { LIGHT0 } else { BRIGHT_BLUE }),
        );
    }

    if response.hovered()
        && let Some(pointer) = response.hover_pos()
        && let Some(index) = (0..*point_count as usize).min_by(|left, right| {
            let left_distance = (x_for_frequency(frequencies_hz[*left]) - pointer.x).abs();
            let right_distance = (x_for_frequency(frequencies_hz[*right]) - pointer.x).abs();
            left_distance.total_cmp(&right_distance)
        })
    {
        let existing_frequency = frequencies_hz[index];
        let near_existing = (x_for_frequency(existing_frequency) - pointer.x).abs() <= 8.0;
        let frequency = if near_existing {
            existing_frequency
        } else {
            frequency_for_x(pointer.x)
        };
        let db = base_at(frequency) + gain_at(frequency, gains_db);
        let hint = if near_existing {
            "Click or drag vertically to adjust this point"
        } else if (*point_count as usize) < EQ_MAX_POINTS {
            "Click to add a point here"
        } else {
            "Point storage is full"
        };
        response.clone().on_hover_text(format!(
            "{}  {db:+.1} dB\n{hint}",
            frequency_label(frequency)
        ));
    }

    committed
}

fn sampled_values(frequency: f32, frequencies: &[f32], values: &[f32]) -> f32 {
    if frequency <= frequencies[0] {
        return values[0];
    }
    if frequency >= *frequencies.last().unwrap() {
        return *values.last().unwrap();
    }
    for index in 0..frequencies.len() - 1 {
        if frequency <= frequencies[index + 1] {
            let low = frequencies[index].ln();
            let high = frequencies[index + 1].ln();
            if high <= low {
                continue;
            }
            let amount = (frequency.ln() - low) / (high - low);
            return values[index] + (values[index + 1] - values[index]) * amount;
        }
    }
    *values.last().unwrap()
}

fn insert_eq_point(
    point_count: &mut u8,
    frequencies_hz: &mut [f32; EQ_MAX_POINTS],
    gains_db: &mut [f32; EQ_MAX_POINTS],
    frequency: f32,
    gain_db: f32,
) -> Option<usize> {
    let count = *point_count as usize;
    if count >= EQ_MAX_POINTS {
        return None;
    }
    let frequency = frequency.clamp(20.0, 20_000.0);
    let insertion = frequencies_hz[..count].partition_point(|value| *value < frequency);
    for index in (insertion..count).rev() {
        frequencies_hz[index + 1] = frequencies_hz[index];
        gains_db[index + 1] = gains_db[index];
    }
    frequencies_hz[insertion] = frequency;
    gains_db[insertion] = gain_db.clamp(-12.0, 12.0);
    *point_count += 1;
    Some(insertion)
}

fn frequency_label(frequency: f32) -> String {
    if frequency >= 1_000.0 {
        let khz = frequency / 1_000.0;
        if khz.fract().abs() < 0.05 {
            format!("{khz:.0}k")
        } else {
            format!("{khz:.1}k")
        }
    } else {
        format!("{frequency:.0}")
    }
}

#[cfg(test)]
mod eq_control_tests {
    use super::*;

    #[test]
    fn inserts_a_new_point_at_its_frequency() {
        let mut count = EQ_BAND_COUNT as u8;
        let mut frequencies = EqProfile::default_frequencies();
        let mut gains = [0.0; EQ_MAX_POINTS];

        let inserted = insert_eq_point(&mut count, &mut frequencies, &mut gains, 750.0, 4.5);

        let inserted = inserted.unwrap();
        assert_eq!(count, 32);
        assert_eq!(frequencies[inserted], 750.0);
        assert_eq!(gains[inserted], 4.5);
    }
}

fn title_bar(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(CONTENT_WIDTH, 32.0), egui::Sense::hover());

    paint_logo(ui.painter(), rect.left_center() + egui::vec2(24.0, 0.0));
    ui.painter().text(
        rect.left_center() + egui::vec2(44.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "ŚAQ",
        egui::FontId::new(16.0, egui::FontFamily::Monospace),
        LIGHT0,
    );
}

fn paint_logo(painter: &egui::Painter, center: egui::Pos2) {
    const PIXEL: f32 = 2.0;
    const SUN: [&str; 12] = [
        "..########..",
        ".##########.",
        "############",
        "############",
        "############",
        "############",
        "############",
        "############",
        "############",
        "############",
        ".##########.",
        "..########..",
    ];
    const SAQ: [&str; 12] = [
        "............",
        "............",
        "..########..",
        ".###....###.",
        "...#....#...",
        "...#....#...",
        "...#....#...",
        "...#....#...",
        "...#....#...",
        "...#....#...",
        "...#....#...",
        "............",
    ];
    let origin = center - egui::vec2(12.0, 12.0);
    for (pattern, color) in [(SUN, BRIGHT_ORANGE), (SAQ, LOGO_BLACK)] {
        for (y, row) in pattern.iter().enumerate() {
            for (x, pixel) in row.bytes().enumerate() {
                if pixel == b'#' {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            origin + egui::vec2(x as f32 * PIXEL, y as f32 * PIXEL),
                            egui::vec2(PIXEL, PIXEL),
                        ),
                        0,
                        color,
                    );
                }
            }
        }
    }
}

fn volume_control(ui: &mut egui::Ui, volume: &mut f32) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(CONTENT_WIDTH, 42.0), egui::Sense::hover());

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(12.0, 6.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.label(
                egui::RichText::new("VOL")
                    .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                    .color(LIGHT3),
            );
            ui.add_space(6.0);

            let (slider_rect, response) =
                ui.allocate_exact_size(egui::vec2(500.0, 24.0), egui::Sense::drag());
            let mut response = response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);

            let drag = response.drag_delta().x;
            if drag != 0.0 {
                let track_width = slider_rect.width() - 10.0;
                let next_volume = if drag < 0.0 {
                    let position = (volume_visual_position(*volume) + drag / track_width).max(0.0);
                    volume_from_visual_position(position)
                } else {
                    let sensitivity = if *volume < 1.0 {
                        2.0 / track_width
                    } else if *volume < 2.0 {
                        4.0 / track_width
                    } else {
                        4.0 / track_width / (1.0 + *volume - 2.0)
                    };
                    *volume + drag * sensitivity
                };
                if next_volume.to_bits() != volume.to_bits() {
                    *volume = next_volume;
                    response.mark_changed();
                }
            }

            let track = egui::Rect::from_center_size(
                slider_rect.center(),
                egui::vec2(slider_rect.width() - 10.0, 4.0),
            );
            let handle_x = egui::lerp(
                track.left()..=track.right(),
                volume_visual_position(*volume),
            );
            ui.painter().rect_filled(track, 0, DARK2);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(track.left_top(), egui::pos2(handle_x, track.bottom())),
                0,
                BRIGHT_AQUA,
            );
            ui.painter().rect_filled(
                egui::Rect::from_center_size(
                    egui::pos2(handle_x, slider_rect.center().y),
                    egui::vec2(8.0, 14.0),
                ),
                0,
                if response.hovered() { LIGHT0 } else { LIGHT1 },
            );
            response.widget_info(|| egui::WidgetInfo::slider(true, *volume as f64, "Volume"));

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(volume_percent(*volume))
                    .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                    .color(LIGHT1),
            );
            response
        },
    )
    .inner
}

fn volume_visual_position(volume: f32) -> f32 {
    const OVERDRIVE_CURVE: f32 = 0.08;

    if volume <= 1.0 {
        volume * 0.5
    } else if volume <= 2.0 {
        0.5 + (volume - 1.0) * 0.25
    } else {
        0.75 + 0.25 * (1.0 - (OVERDRIVE_CURVE * (2.0 - volume)).exp())
    }
}

fn volume_from_visual_position(position: f32) -> f32 {
    const OVERDRIVE_CURVE: f32 = 0.08;

    if position <= 0.5 {
        position * 2.0
    } else if position <= 0.75 {
        1.0 + (position - 0.5) * 4.0
    } else {
        2.0 - (1.0 - (position - 0.75) * 4.0).ln() / OVERDRIVE_CURVE
    }
}

fn volume_percent(volume: f32) -> String {
    let percent = volume as f64 * 100.0;
    if percent < 10_000.0 {
        format!("{percent:>3.0}%")
    } else {
        format!("{percent:.1e}%")
    }
}

#[derive(Clone, Copy)]
enum ModeIcon {
    Spatial,
    Stereo,
    Orbit,
    Cube,
    Room,
    Clarity,
    Night,
}

fn toggle(selected: Mode, clicked: Mode) -> Mode {
    if selected == clicked {
        Mode::Off
    } else {
        clicked
    }
}

fn mode_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: ModeIcon,
    accent: egui::Color32,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let icon_color = if selected || response.hovered() {
        accent
    } else {
        LIGHT3
    };
    paint_pixel_icon(
        ui.painter(),
        egui::pos2(rect.center().x, rect.top() + 34.0),
        icon.pattern(),
        icon_color,
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 14.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(9.0, egui::FontFamily::Monospace),
        if selected { LIGHT0 } else { LIGHT1 },
    );
    response
}

impl ModeIcon {
    fn pattern(self) -> &'static [&'static str] {
        match self {
            Self::Spatial => &[
                "#...#...#",
                ".#..#..#.",
                "..#.#.#..",
                "...###...",
                "####.####",
                "...###...",
                "..#.#.#..",
                ".#..#..#.",
                "#...#...#",
            ],
            Self::Stereo => &[
                ".###...###.",
                ".#.#...#.#.",
                ".###...###.",
                ".#.#...#.#.",
                ".###...###.",
                ".#.#...#.#.",
                ".###...###.",
            ],
            Self::Orbit => &[
                "..#####..",
                ".#.....#.",
                "#..###..#",
                "#.#...#.#",
                "#.#.#.#.#",
                "#.#...#.#",
                "#..###..#",
                ".#.....#.",
                "..#####..",
            ],
            Self::Cube => &[
                "....#....",
                "..##.##..",
                "##.....##",
                "#.#...#.#",
                "#..#.#..#",
                "#...#...#",
                "##..#..##",
                "..#####..",
                "....#....",
            ],
            Self::Room => &[
                "......#..",
                ".......#.",
                "....#..#.",
                ".....#..#",
                "..#..#..#",
                ".....#..#",
                "....#..#.",
                ".......#.",
                "......#..",
            ],
            Self::Clarity => &[
                "......#..",
                "..#...#..",
                "..#..#.#.",
                ".#.#.#.#.",
                ".#.#.#..#",
                "#...#...#",
                "#.......#",
            ],
            Self::Night => &[
                ".........",
                "...#####.",
                "..###....",
                "..##.....",
                "..##.....",
                "..##.....",
                "..###....",
                "...#####.",
                ".........",
            ],
        }
    }
}

fn paint_pixel_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    pattern: &[&str],
    color: egui::Color32,
) {
    const PIXEL: f32 = 3.0;
    let width = pattern.first().map_or(0, |row| row.len()) as f32 * PIXEL;
    let height = pattern.len() as f32 * PIXEL;
    let origin = center - egui::vec2(width, height) * 0.5;
    for (y, row) in pattern.iter().enumerate() {
        for (x, pixel) in row.bytes().enumerate() {
            if pixel == b'#' {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        origin + egui::vec2(x as f32 * PIXEL, y as f32 * PIXEL),
                        egui::vec2(PIXEL, PIXEL),
                    ),
                    0,
                    color,
                );
            }
        }
    }
}

fn surround_sound_control(ui: &mut egui::Ui, selected: bool, subwoofer: &mut f32) -> (bool, bool) {
    use std::f32::consts::{FRAC_PI_2, PI};

    const HALF_ARC: f32 = PI * 0.75;
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, egui::Sense::click_and_drag());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = egui::pos2(rect.center().x, rect.top() + 35.0);
    let pointer = response.interact_pointer_pos();
    let pointer_radius = pointer.map_or(0.0, |position| position.distance(center));
    let mode_clicked = response.clicked() && (!selected || pointer_radius < 16.0);
    let mut subwoofer_changed = false;

    if selected
        && pointer_radius >= 16.0
        && (response.clicked() || response.dragged())
        && let Some(pointer) = pointer
    {
        let value = dial_position(pointer - center);
        subwoofer_changed = value != *subwoofer;
        *subwoofer = value;
    }

    let marker_angle = subwoofer.clamp(0.0, 1.0) * HALF_ARC * 2.0 - FRAC_PI_2 - HALF_ARC;
    let color = if selected || response.hovered() {
        BRIGHT_ORANGE
    } else {
        LIGHT3
    };
    paint_pitch_wheel(ui.painter(), center, marker_angle, *subwoofer, false, color);
    paint_pixel_icon(ui.painter(), center, ModeIcon::Cube.pattern(), color);
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 14.0),
        egui::Align2::CENTER_CENTER,
        "SURROUND SOUND",
        egui::FontId::new(9.0, egui::FontFamily::Monospace),
        if selected { LIGHT0 } else { LIGHT1 },
    );

    (mode_clicked, subwoofer_changed)
}

fn dial_position(direction: egui::Vec2) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI};

    const HALF_ARC: f32 = PI * 0.75;
    let mut angle = direction.y.atan2(direction.x) + FRAC_PI_2;
    if angle > PI {
        angle -= 2.0 * PI;
    } else if angle < -PI {
        angle += 2.0 * PI;
    }
    (angle.clamp(-HALF_ARC, HALF_ARC) / HALF_ARC + 1.0) * 0.5
}

fn pitch_control(ui: &mut egui::Ui, enabled: &mut bool, semitones: &mut f32) -> egui::Response {
    use std::f32::consts::{FRAC_PI_2, PI};

    const HALF_ARC: f32 = PI * 0.75;
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, egui::Sense::click_and_drag());
    let mut response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = egui::pos2(rect.center().x, rect.top() + 35.0);
    let pointer = response.interact_pointer_pos();
    let pointer_radius = pointer.map_or(0.0, |position| position.distance(center));
    let was_enabled = *enabled;

    if response.clicked() {
        response.mark_changed();
        if !was_enabled {
            *enabled = true;
        } else if pointer_radius < 16.0 {
            *enabled = false;
        }
    }
    if was_enabled
        && *enabled
        && pointer_radius >= 16.0
        && (response.clicked() || response.dragged())
        && let Some(pointer) = pointer
    {
        *semitones = pitch_from_direction(pointer - center);
        response.mark_changed();
    }

    let marker_offset = semitones.clamp(-12.0, 12.0) / 12.0 * HALF_ARC;
    let marker_angle = marker_offset - FRAC_PI_2;
    let gradient_position = (semitones.clamp(-12.0, 12.0) + 12.0) / 24.0;
    let marker_color = paint_pitch_wheel(
        ui.painter(),
        center,
        marker_angle,
        gradient_position,
        *enabled || response.hovered(),
        LIGHT3,
    );
    paint_pitch_arrows(ui.painter(), center, marker_color);
    let label = if *enabled {
        format!("PITCH {:+.1}", *semitones)
    } else {
        "PITCH".to_owned()
    };
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 14.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(9.0, egui::FontFamily::Monospace),
        if *enabled { LIGHT0 } else { LIGHT1 },
    );
    response
}

fn pitch_from_direction(direction: egui::Vec2) -> f32 {
    let semitones = dial_position(direction) * 24.0 - 12.0;
    if semitones.abs() <= 0.25 {
        0.0
    } else {
        semitones
    }
}

fn paint_pitch_wheel(
    painter: &egui::Painter,
    center: egui::Pos2,
    marker_angle: f32,
    gradient_position: f32,
    show_gradient: bool,
    plain_color: egui::Color32,
) -> egui::Color32 {
    use std::f32::consts::{FRAC_PI_2, PI};

    const ARC_RADIUS: f32 = 25.0;
    const HALF_ARC: f32 = PI * 0.75;
    const ARC_SEGMENTS: usize = 64;
    let start = -FRAC_PI_2 - HALF_ARC;

    let pitch_gradient = |amount: f32| -> egui::Color32 {
        let to_oklab = |color: egui::Color32| {
            Oklab::from_color(Srgb::new(
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
            ))
        };
        let color = to_oklab(BRIGHT_BLUE).mix(to_oklab(BRIGHT_RED), amount);
        let color: Srgb<u8> = Srgb::from_color(color).clamp().into_format();
        egui::Color32::from_rgb(color.red, color.green, color.blue)
    };

    for segment in 0..ARC_SEGMENTS {
        let from_progress = segment as f32 / ARC_SEGMENTS as f32;
        let to_progress = (segment + 1) as f32 / ARC_SEGMENTS as f32;
        let point = |progress: f32| {
            let angle = start + progress * HALF_ARC * 2.0;
            center + egui::vec2(angle.cos(), angle.sin()) * ARC_RADIUS
        };
        let color = if show_gradient {
            pitch_gradient(from_progress)
        } else {
            plain_color
        };
        painter.line_segment(
            [point(from_progress), point(to_progress)],
            egui::Stroke::new(2.0_f32, color),
        );
    }

    let marker_color = if show_gradient {
        pitch_gradient(gradient_position)
    } else {
        plain_color
    };
    paint_pitch_handle(painter, center, marker_angle, marker_color);
    marker_color
}

fn paint_pitch_handle(
    painter: &egui::Painter,
    center: egui::Pos2,
    angle: f32,
    color: egui::Color32,
) {
    const RADIUS: f32 = 25.0;
    const HALF_LENGTH: f32 = 5.0;
    const HALF_WIDTH: f32 = 3.0;
    let radial = egui::vec2(angle.cos(), angle.sin());
    let tangent = egui::vec2(-radial.y, radial.x);
    let block_center = center + radial * RADIUS;
    painter.add(egui::Shape::convex_polygon(
        vec![
            block_center - tangent * HALF_LENGTH - radial * HALF_WIDTH,
            block_center + tangent * HALF_LENGTH - radial * HALF_WIDTH,
            block_center + tangent * HALF_LENGTH + radial * HALF_WIDTH,
            block_center - tangent * HALF_LENGTH + radial * HALF_WIDTH,
        ],
        color,
        egui::Stroke::new(1.0_f32, DARK0),
    ));
}

fn paint_pitch_arrows(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    const PIXEL: f32 = 2.0;
    const PATTERN: [&str; 7] = [
        ".#....#..",
        "###...#..",
        ".#....#..",
        ".#....#..",
        ".#....#..",
        ".#...###.",
        ".#....#..",
    ];
    let width = PATTERN[0].len() as f32 * PIXEL;
    let height = PATTERN.len() as f32 * PIXEL;
    let origin = center - egui::vec2(width, height) * 0.5;
    for (y, row) in PATTERN.iter().enumerate() {
        for (x, pixel) in row.bytes().enumerate() {
            if pixel == b'#' {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        origin + egui::vec2(x as f32 * PIXEL, y as f32 * PIXEL),
                        egui::vec2(PIXEL, PIXEL),
                    ),
                    0,
                    color,
                );
            }
        }
    }
}
