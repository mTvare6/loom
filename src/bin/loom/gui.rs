// SPDX-License-Identifier: MPL-2.0

use eframe::egui;
use loom_dsp::Mode;
use loom_ipc::{Event, IpcClient, Request, Response};
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
const WINDOW_SIZE: egui::Vec2 = egui::vec2(708.0, 350.0);

// FIXME: Rewrite it with hand and understand GUI options
pub fn run_gui(socket: impl AsRef<Path>) -> eframe::Result<()> {
    let mut ipc_client = match IpcClient::new(socket) {
        Ok(client) => client,
        Err(error) => panic!("{:?}", error),
    };

    let state = ipc_client
        .send(Request::GetState)
        .expect("Failed to get state");

    let (volume, mode, pitch_enabled, pitch_semitones, subwoofer) = match state {
        Response::State {
            volume,
            mode,
            pitch_enabled,
            pitch,
            subwoofer,
        } => (volume, Mode::from_u8(mode), pitch_enabled, pitch, subwoofer),
        _ => unreachable!(),
    };

    eframe::run_native(
        "Loom",
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
            Ok(Box::new(LoomApp {
                ipc_client,
                volume,
                mode,
                pitch_enabled,
                pitch_semitones,
                subwoofer,
                window_size_initialized: false,
            }))
        }),
    )
}

fn handle_response(response: std::io::Result<Response>) {
    match response {
        Err(err) => {
            eprintln!("Loom daemon error: {}", err)
        }
        Ok(Response::Error(error)) => {
            eprintln!("Error: {}", error);
        }
        _ => {}
    }
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

struct LoomApp {
    ipc_client: IpcClient,
    volume: f32,
    mode: Mode,
    pitch_enabled: bool,
    pitch_semitones: f32,
    subwoofer: f32,
    window_size_initialized: bool,
}

impl LoomApp {
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
                } => {
                    self.volume = volume;
                    self.mode = Mode::from_u8(mode);
                    self.pitch_enabled = pitch_enabled;
                    self.pitch_semitones = pitch;
                    self.subwoofer = subwoofer;
                    ctx.request_repaint();
                }
            }
        }
    }
}

impl eframe::App for LoomApp {
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

                let (
                    (surround_clicked, subwoofer_changed),
                    spatial_filter,
                    spatial_stereo,
                    spatial_surround,
                ) = ui
                    .horizontal(|ui| {
                        (
                            surround_control(
                                ui,
                                self.mode == Mode::Surround3d,
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

                let (ambience, fidelity, night, pitch) = ui
                    .horizontal(|ui| {
                        (
                            mode_button(
                                ui,
                                "AMBIENCE",
                                ModeIcon::Ambience,
                                BRIGHT_GREEN,
                                self.mode == Mode::Ambience,
                            ),
                            mode_button(
                                ui,
                                "FIDELITY",
                                ModeIcon::Fidelity,
                                BRIGHT_YELLOW,
                                self.mode == Mode::Fidelity,
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

                let new_mode = if surround_clicked {
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

fn title_bar(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(CONTENT_WIDTH, 32.0), egui::Sense::hover());

    paint_logo(ui.painter(), rect.left_center() + egui::vec2(24.0, 0.0));
    ui.painter().text(
        rect.left_center() + egui::vec2(44.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "LOOM",
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
    const LOOM: [&str; 12] = [
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
    for (pattern, color) in [(SUN, BRIGHT_ORANGE), (LOOM, LOGO_BLACK)] {
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
            Self::Ambience => &[
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
            Self::Fidelity => &[
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

fn surround_control(ui: &mut egui::Ui, selected: bool, subwoofer: &mut f32) -> (bool, bool) {
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
        "3D SURROUND",
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
