// SPDX-License-Identifier: MPL-2.0

use saq_dsp::Mode;
use saq_ipc::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

// FIXME: State graph should allow more complicate transition
// structure. Clarity should be valid across different modes
// and a two-state solution is neccesary.
#[derive(Serialize, Deserialize)]
pub struct AudioState {
    volume: AtomicU32,
    mode: AtomicU8,
    pitch_enabled: AtomicBool,
    pitch_semitones: AtomicU32,
}

impl AudioState {
    pub fn new(initial_volume: f32) -> Self {
        Self {
            volume: AtomicU32::new(initial_volume.to_bits()),
            mode: AtomicU8::new(Mode::SurroundSound as u8),
            pitch_enabled: AtomicBool::new(false),
            pitch_semitones: AtomicU32::new(0.0_f32.to_bits()),
        }
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
    pub fn set_volume(&self, vol: f32) {
        self.volume.store(vol.to_bits(), Ordering::Relaxed);
    }

    pub fn mode(&self) -> Mode {
        Mode::from_u8(self.mode.load(Ordering::Relaxed))
    }
    pub fn set_mode(&self, mode: Mode) {
        self.mode.store(mode as u8, Ordering::Relaxed);
    }

    pub fn pitch_enabled(&self) -> bool {
        self.pitch_enabled.load(Ordering::Relaxed)
    }

    pub fn set_pitch_enabled(&self, enabled: bool) {
        self.pitch_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn pitch_semitones(&self) -> f32 {
        f32::from_bits(self.pitch_semitones.load(Ordering::Relaxed))
    }

    pub fn set_pitch_semitones(&self, semitones: f32) {
        self.pitch_semitones
            .store(semitones.clamp(-12.0, 12.0).to_bits(), Ordering::Relaxed);
    }

    pub fn handle_query(&self, request: Request) -> Response {
        match request {
            Request::SetVolume(volume) => {
                self.set_volume(volume);
                Response::Ok
            }
            Request::SetMode(mode) => {
                self.set_mode(Mode::from_u8(mode));
                Response::Ok
            }
            Request::GetState => Response::State {
                volume: self.volume(),
                mode: self.mode() as u8,
                pitch_enabled: self.pitch_enabled(),
                pitch: self.pitch_semitones(),
            },
            Request::SetPitchEnabled(pitch_enabled) => {
                self.set_pitch_enabled(pitch_enabled);
                Response::Ok
            }
            Request::SetPitch(pitch) => {
                if self.pitch_enabled() {
                    self.set_pitch_semitones(pitch);
                    Response::Ok
                } else {
                    Response::Error
                }
            }
        }
    }
}

impl saq_pipewire::AudioControls for AudioState {
    fn volume(&self) -> f32 {
        AudioState::volume(self)
    }

    fn mode(&self) -> Mode {
        AudioState::mode(self)
    }

    fn pitch_enabled(&self) -> bool {
        AudioState::pitch_enabled(self)
    }

    fn pitch_semitones(&self) -> f32 {
        AudioState::pitch_semitones(self)
    }
}
