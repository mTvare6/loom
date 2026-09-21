// SPDX-License-Identifier: MPL-2.0

use saq_dsp::{EQ_BAND_COUNT, EQ_MAX_POINTS, EqPreset, EqProfile, Mode};
use saq_ipc::{Event, Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

// FIXME: State graph should allow more complicate transition
// structure. Clarity should be valid across different modes
// and a two-state solution is neccesary.
#[derive(Serialize, Deserialize)]
pub struct AudioState {
    #[serde(with = "atomic_f32")]
    volume: AtomicU32,
    mode: AtomicU8,
    pitch_enabled: AtomicBool,
    #[serde(with = "atomic_f32")]
    pitch_semitones: AtomicU32,
    #[serde(with = "atomic_f32")]
    subwoofer: AtomicU32,
    eq_preset: AtomicU8,
    eq_base_preset: AtomicU8,
    eq_point_count: AtomicU8,
    #[serde(with = "atomic_f32_array")]
    eq_frequencies_hz: [AtomicU32; EQ_MAX_POINTS],
    #[serde(with = "atomic_f32_array")]
    eq_gains_db: [AtomicU32; EQ_MAX_POINTS],
}

mod atomic_f32 {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::sync::atomic::{AtomicU32, Ordering};

    pub fn serialize<S>(value: &AtomicU32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f32(f32::from_bits(value.load(Ordering::Relaxed)))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AtomicU32, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(AtomicU32::new(f32::deserialize(deserializer)?.to_bits()))
    }
}

mod atomic_f32_array {
    use super::EQ_MAX_POINTS;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::sync::atomic::{AtomicU32, Ordering};

    pub fn serialize<S>(
        values: &[AtomicU32; EQ_MAX_POINTS],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        values
            .iter()
            .map(|value| f32::from_bits(value.load(Ordering::Relaxed)))
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[AtomicU32; EQ_MAX_POINTS], D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = Vec::<f32>::deserialize(deserializer)?;
        Ok(std::array::from_fn(|index| {
            AtomicU32::new(values.get(index).copied().unwrap_or(0.0).to_bits())
        }))
    }
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            volume: AtomicU32::new(1.0_f32.to_bits()),
            mode: AtomicU8::new(Mode::default() as u8),
            pitch_enabled: AtomicBool::new(false),
            pitch_semitones: AtomicU32::new(0.0_f32.to_bits()),
            subwoofer: AtomicU32::new(1.0_f32.to_bits()),
            eq_preset: AtomicU8::new(EqPreset::Off as u8),
            eq_base_preset: AtomicU8::new(EqPreset::Off as u8),
            eq_point_count: AtomicU8::new(EQ_BAND_COUNT as u8),
            eq_frequencies_hz: EqProfile::default_frequencies()
                .map(|frequency| AtomicU32::new(frequency.to_bits())),
            eq_gains_db: std::array::from_fn(|_| AtomicU32::default()),
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

    pub fn subwoofer(&self) -> f32 {
        let stored = self.subwoofer.load(Ordering::Relaxed);
        if stored <= 100 {
            stored as f32 / 100.0
        } else {
            f32::from_bits(stored)
        }
    }

    pub fn set_subwoofer(&self, value: f32) {
        if value.is_finite() {
            self.subwoofer
                .store(value.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        }
    }

    pub fn eq_preset(&self) -> EqPreset {
        EqPreset::from_u8(self.eq_preset.load(Ordering::Relaxed))
    }

    pub fn set_eq_preset(&self, preset: EqPreset) {
        self.eq_preset.store(preset as u8, Ordering::Relaxed);
        if preset != EqPreset::Custom {
            self.eq_base_preset.store(preset as u8, Ordering::Relaxed);
            self.eq_point_count
                .store(EQ_BAND_COUNT as u8, Ordering::Relaxed);
            for (target, frequency) in self
                .eq_frequencies_hz
                .iter()
                .zip(EqProfile::default_frequencies())
            {
                target.store(frequency.to_bits(), Ordering::Relaxed);
            }
            for gain in &self.eq_gains_db {
                gain.store(0.0_f32.to_bits(), Ordering::Relaxed);
            }
        }
    }

    pub fn eq_profile(&self) -> EqProfile {
        EqProfile {
            preset: self.eq_preset(),
            base_preset: EqPreset::from_u8(self.eq_base_preset.load(Ordering::Relaxed)),
            point_count: self.eq_point_count.load(Ordering::Relaxed),
            frequencies_hz: std::array::from_fn(|index| {
                f32::from_bits(self.eq_frequencies_hz[index].load(Ordering::Relaxed))
            }),
            gains_db: std::array::from_fn(|index| {
                f32::from_bits(self.eq_gains_db[index].load(Ordering::Relaxed))
            }),
        }
        .sanitized()
    }

    pub fn set_eq_profile(&self, profile: EqProfile) {
        let profile = profile.sanitized();
        self.eq_base_preset
            .store(profile.base_preset as u8, Ordering::Relaxed);
        self.eq_point_count
            .store(profile.point_count, Ordering::Relaxed);
        for (target, frequency) in self.eq_frequencies_hz.iter().zip(profile.frequencies_hz) {
            target.store(frequency.to_bits(), Ordering::Relaxed);
        }
        for (target, gain) in self.eq_gains_db.iter().zip(profile.gains_db) {
            target.store(gain.to_bits(), Ordering::Relaxed);
        }
        self.eq_preset
            .store(profile.preset as u8, Ordering::Release);
    }

    // TODO: Return an error for volume out of bounds and for invalid mode
    pub fn handle_query(&self, request: Request) -> (Response, Option<Event>) {
        match request {
            Request::SetVolume(volume) => {
                self.set_volume(volume);
                (Response::Ok, Some(self.updated_event()))
            }
            Request::SetMode(mode) => {
                self.set_mode(Mode::from_u8(mode));
                (Response::Ok, Some(self.updated_event()))
            }
            Request::GetState => {
                let eq_profile = self.eq_profile();
                (
                    Response::State {
                        volume: self.volume(),
                        mode: self.mode() as u8,
                        pitch_enabled: self.pitch_enabled(),
                        pitch: self.pitch_semitones(),
                        subwoofer: self.subwoofer(),
                        eq_preset: eq_profile.preset as u8,
                        eq_base_preset: eq_profile.base_preset as u8,
                        eq_point_count: eq_profile.point_count,
                        eq_frequencies_hz: eq_profile.frequencies_hz.to_vec(),
                        eq_gains_db: eq_profile.gains_db.to_vec(),
                    },
                    None,
                )
            }
            Request::SetPitchEnabled(pitch_enabled) => {
                self.set_pitch_enabled(pitch_enabled);
                (Response::Ok, Some(self.updated_event()))
            }
            Request::SetPitch(pitch) => {
                if self.pitch_enabled() {
                    self.set_pitch_semitones(pitch);
                    (Response::Ok, Some(self.updated_event()))
                } else {
                    (
                        Response::Error("Tried to set pitch but pitch is not enabled".to_string()),
                        None,
                    )
                }
            }
            Request::SetSubwoofer(value) => {
                self.set_subwoofer(value);
                (Response::Ok, Some(self.updated_event()))
            }
            Request::SetEqPreset(preset) => {
                self.set_eq_preset(EqPreset::from_u8(preset));
                (Response::Ok, Some(self.updated_event()))
            }
            Request::SetEqProfile {
                preset,
                base_preset,
                point_count,
                frequencies_hz,
                gains_db,
            } => {
                // Simpler and requires lesser messages like Add/Move/Remove point
                let default_frequencies = EqProfile::default_frequencies();
                self.set_eq_profile(EqProfile {
                    preset: EqPreset::from_u8(preset),
                    base_preset: EqPreset::from_u8(base_preset),
                    point_count,
                    frequencies_hz: std::array::from_fn(|index| {
                        frequencies_hz
                            .get(index)
                            .copied()
                            .unwrap_or(default_frequencies[index])
                    }),
                    gains_db: std::array::from_fn(|index| {
                        gains_db.get(index).copied().unwrap_or(0.0)
                    }),
                });
                (Response::Ok, Some(self.updated_event()))
            }
        }
    }

    fn updated_event(&self) -> Event {
        let eq_profile = self.eq_profile();
        Event::StateUpdated {
            volume: self.volume(),
            mode: self.mode() as u8,
            pitch_enabled: self.pitch_enabled(),
            pitch: self.pitch_semitones(),
            subwoofer: self.subwoofer(),
            eq_preset: eq_profile.preset as u8,
            eq_base_preset: eq_profile.base_preset as u8,
            eq_point_count: eq_profile.point_count,
            eq_frequencies_hz: eq_profile.frequencies_hz.to_vec(),
            eq_gains_db: eq_profile.gains_db.to_vec(),
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

    fn subwoofer(&self) -> f32 {
        AudioState::subwoofer(self)
    }

    fn eq_profile(&self) -> EqProfile {
        AudioState::eq_profile(self)
    }
}
