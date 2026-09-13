// SPDX-License-Identifier: MPL-2.0

use crate::state::AudioState;
use eframe::egui;
use saq_dsp::Mode;
use palette::{Clamp, FromColor, Mix, Oklab, Srgb};
use std::sync::Arc;

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

pub fn run_gui(state: Arc<AudioState>) -> eframe::Result<()> {
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
                state,
                window_size_initialized: false,
            }))
        }),
    )
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
    state: Arc<AudioState>,
    window_size_initialized: bool,
}

impl eframe::App for SaqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.window_size_initialized {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(WINDOW_SIZE));
            self.window_size_initialized = true;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(ui.max_rect(), 0, DARK0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(CONTENT_WIDTH);
                title_bar(ui);

                let mut volume = self.state.volume();
                volume_control(ui, &mut volume);
                self.state.set_volume(volume);

                let selected = self.state.mode();
                let (surround, spatial_filter, spatial_stereo, spatial_surround) = ui
                    .horizontal(|ui| {
                        (
                            mode_button(
                                ui,
                                "SURROUND SOUND",
                                ModeIcon::Cube,
                                BRIGHT_ORANGE,
                                selected == Mode::SurroundSound,
                            ),
                            mode_button(
                                ui,
                                "SPATIAL FILTER",
                                ModeIcon::Spatial,
                                BRIGHT_AQUA,
                                selected == Mode::SpatialFilter,
                            ),
                            mode_button(
                                ui,
                                "SPATIAL STEREO",
                                ModeIcon::Stereo,
                                BRIGHT_BLUE,
                                selected == Mode::SpatialStereo,
                            ),
                            mode_button(
                                ui,
                                "SPATIAL SURROUND",
                                ModeIcon::Orbit,
                                BRIGHT_PURPLE,
                                selected == Mode::SpatialSurround,
                            ),
                        )
                    })
                    .inner;

                let mut pitch_enabled = self.state.pitch_enabled();
                let mut pitch_semitones = self.state.pitch_semitones();
                let (room, clarity, night, _pitch) = ui
                    .horizontal(|ui| {
                        (
                            mode_button(
                                ui,
                                "ROOM",
                                ModeIcon::Room,
                                BRIGHT_GREEN,
                                selected == Mode::Room,
                            ),
                            mode_button(
                                ui,
                                "CLARITY",
                                ModeIcon::Clarity,
                                BRIGHT_YELLOW,
                                selected == Mode::Clarity,
                            ),
                            mode_button(
                                ui,
                                "NIGHT",
                                ModeIcon::Night,
                                BRIGHT_RED,
                                selected == Mode::Night,
                            ),
                            pitch_control(ui, &mut pitch_enabled, &mut pitch_semitones),
                        )
                    })
                    .inner;

                if surround.clicked() {
                    self.state.set_mode(toggle(selected, Mode::SurroundSound));
                } else if spatial_filter.clicked() {
                    self.state.set_mode(toggle(selected, Mode::SpatialFilter));
                } else if spatial_stereo.clicked() {
                    self.state.set_mode(toggle(selected, Mode::SpatialStereo));
                } else if spatial_surround.clicked() {
                    self.state.set_mode(toggle(selected, Mode::SpatialSurround));
                } else if room.clicked() {
                    self.state.set_mode(toggle(selected, Mode::Room));
                } else if clarity.clicked() {
                    self.state.set_mode(toggle(selected, Mode::Clarity));
                } else if night.clicked() {
                    self.state.set_mode(toggle(selected, Mode::Night));
                }
                self.state.set_pitch_enabled(pitch_enabled);
                self.state.set_pitch_semitones(pitch_semitones);
            });
        });
        ctx.request_repaint();
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

            let response = ui.add_sized(
                [532.0, 24.0],
                egui::Slider::new(volume, 0.0..=2.0)
                    .show_value(false)
                    .trailing_fill(true),
            );

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("{:>3}%", (*volume * 100.0).round() as i32))
                    .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                    .color(LIGHT1),
            );
            response
        },
    )
    .inner
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

fn pitch_control(ui: &mut egui::Ui, enabled: &mut bool, semitones: &mut f32) -> egui::Response {
    use std::f32::consts::{FRAC_PI_2, PI};

    const HALF_ARC: f32 = PI * 0.75;
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, egui::Sense::click_and_drag());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = egui::pos2(rect.center().x, rect.top() + 35.0);
    let pointer = response.interact_pointer_pos();
    let pointer_radius = pointer.map_or(0.0, |position| position.distance(center));
    let was_enabled = *enabled;

    if response.clicked() {
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
    }

    let marker_offset = semitones.clamp(-12.0, 12.0) / 12.0 * HALF_ARC;
    let marker_angle = marker_offset - FRAC_PI_2;
    let gradient_position = (semitones.clamp(-12.0, 12.0) + 12.0) / 24.0;
    paint_pitch_wheel(
        ui.painter(),
        center,
        marker_angle,
        gradient_position,
        *enabled || response.hovered(),
    );
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
    use std::f32::consts::{FRAC_PI_2, PI};

    const HALF_ARC: f32 = PI * 0.75;
    let mut angle = direction.y.atan2(direction.x) + FRAC_PI_2;
    if angle > PI {
        angle -= 2.0 * PI;
    } else if angle < -PI {
        angle += 2.0 * PI;
    }
    let semitones = angle.clamp(-HALF_ARC, HALF_ARC) / HALF_ARC * 12.0;
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
) {
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
            LIGHT3
        };
        painter.line_segment(
            [point(from_progress), point(to_progress)],
            egui::Stroke::new(2.0_f32, color),
        );
    }

    let marker_color = if show_gradient {
        pitch_gradient(gradient_position)
    } else {
        LIGHT3
    };
    paint_pitch_handle(painter, center, marker_angle, marker_color);
    paint_pitch_arrows(painter, center, marker_color);
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
