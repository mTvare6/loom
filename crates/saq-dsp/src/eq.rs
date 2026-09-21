// SPDX-License-Identifier: MPL-2.0

use crate::SAMPLE_RATE;
use crate::convolution::StereoFir;
use crate::fft::{Complex, FourierTransform};

pub const EQ_BAND_COUNT: usize = 31;
pub const EQ_MAX_POINTS: usize = 128;
// TODO: Try to figure the formula. Too many sources report different non-identical approximates
pub const EQ_BAND_FREQUENCIES: [f32; EQ_BAND_COUNT] = [
    20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0,
    500.0, 630.0, 800.0, 1_000.0, 1_250.0, 1_600.0, 2_000.0, 2_500.0, 3_150.0, 4_000.0, 5_000.0,
    6_300.0, 8_000.0, 10_000.0, 12_500.0, 16_000.0, 20_000.0,
];

#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum EqPreset {
    #[default]
    Off = 0,
    Dialogue = 1,
    Acoustic = 2,
    BassBoost = 3,
    Orchestral = 4,
    Party = 5,
    Electronic = 6,
    Gaming = 7,
    HipHop = 8,
    House = 9,
    Jazz = 10,
    Cinema = 11,
    Pop = 12,
    Rock = 13,
    TrebleBoost = 14,
    #[cfg_attr(feature = "cli", value(skip))]
    Custom = 15,
}

impl EqPreset {
    const FILTERS: [Self; 14] = [
        Self::Dialogue,
        Self::Acoustic,
        Self::BassBoost,
        Self::Orchestral,
        Self::Party,
        Self::Electronic,
        Self::Gaming,
        Self::HipHop,
        Self::House,
        Self::Jazz,
        Self::Cinema,
        Self::Pop,
        Self::Rock,
        Self::TrebleBoost,
    ];

    pub const SELECTABLE: [Self; 15] = [
        Self::Off,
        Self::Acoustic,
        Self::BassBoost,
        Self::Orchestral,
        Self::Party,
        Self::Electronic,
        Self::Gaming,
        Self::HipHop,
        Self::House,
        Self::Jazz,
        Self::Cinema,
        Self::Pop,
        Self::Rock,
        Self::TrebleBoost,
        Self::Dialogue,
    ];

    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Dialogue,
            2 => Self::Acoustic,
            3 => Self::BassBoost,
            4 => Self::Orchestral,
            5 => Self::Party,
            6 => Self::Electronic,
            7 => Self::Gaming,
            8 => Self::HipHop,
            9 => Self::House,
            10 => Self::Jazz,
            11 => Self::Cinema,
            12 => Self::Pop,
            13 => Self::Rock,
            14 => Self::TrebleBoost,
            15 => Self::Custom,
            _ => Self::Off,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Dialogue => "DIALOGUE",
            Self::Custom => "CUSTOM",
            Self::Acoustic => "ACOUSTIC",
            Self::BassBoost => "BASS BOOST",
            Self::Party => "PARTY",
            Self::Electronic => "ELECTRONIC",
            Self::Gaming => "GAMING",
            Self::HipHop => "HIP HOP",
            Self::House => "HOUSE",
            Self::Jazz => "JAZZ",
            Self::Cinema => "CINEMA",
            Self::Pop => "POP",
            Self::Rock => "ROCK",
            Self::TrebleBoost => "TREBLE BOOST",
            Self::Orchestral => "ORCHESTRAL",
        }
    }

    pub fn response_db(self, frequencies: &[f32]) -> Vec<f32> {
        match self {
            Self::Off | Self::Custom => vec![0.0; frequencies.len()],
            preset => EqImpulse::for_preset(preset).response_db(frequencies),
        }
    }

    const fn filter_index(self) -> Option<usize> {
        match self {
            Self::Dialogue => Some(0),
            Self::Acoustic => Some(1),
            Self::BassBoost => Some(2),
            Self::Orchestral => Some(3),
            Self::Party => Some(4),
            Self::Electronic => Some(5),
            Self::Gaming => Some(6),
            Self::HipHop => Some(7),
            Self::House => Some(8),
            Self::Jazz => Some(9),
            Self::Cinema => Some(10),
            Self::Pop => Some(11),
            Self::Rock => Some(12),
            Self::TrebleBoost => Some(13),
            Self::Off | Self::Custom => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EqProfile {
    pub preset: EqPreset,
    pub base_preset: EqPreset,
    pub point_count: u8,
    pub frequencies_hz: [f32; EQ_MAX_POINTS],
    pub gains_db: [f32; EQ_MAX_POINTS],
}

impl Default for EqProfile {
    fn default() -> Self {
        Self {
            preset: EqPreset::Off,
            base_preset: EqPreset::Off,
            point_count: EQ_BAND_COUNT as u8,
            frequencies_hz: Self::default_frequencies(),
            gains_db: [0.0; EQ_MAX_POINTS],
        }
    }
}

impl EqProfile {
    pub const fn default_frequencies() -> [f32; EQ_MAX_POINTS] {
        let mut frequencies = [0.0; EQ_MAX_POINTS];
        let mut index = 0;
        while index < EQ_BAND_COUNT {
            frequencies[index] = EQ_BAND_FREQUENCIES[index];
            index += 1;
        }
        frequencies
    }

    pub fn sanitized(mut self) -> Self {
        if self.base_preset == EqPreset::Custom {
            self.base_preset = EqPreset::Off;
        }
        self.point_count = if self.point_count == 0 {
            EQ_BAND_COUNT as u8
        } else {
            self.point_count.min(EQ_MAX_POINTS as u8)
        };
        for gain in &mut self.gains_db {
            *gain = if gain.is_finite() {
                gain.clamp(-12.0, 12.0)
            } else {
                0.0
            };
        }
        for index in 0..self.point_count as usize {
            let frequency = self.frequencies_hz[index];
            self.frequencies_hz[index] = if frequency.is_finite() {
                frequency.clamp(20.0, 20_000.0)
            } else {
                Self::default_frequencies()[index.min(EQ_BAND_COUNT - 1)]
            };
        }
        for index in 1..self.point_count as usize {
            let mut current = index;
            while current > 0 && self.frequencies_hz[current] < self.frequencies_hz[current - 1] {
                self.frequencies_hz.swap(current, current - 1);
                self.gains_db.swap(current, current - 1);
                current -= 1;
            }
        }
        self
    }

    pub fn gain_at(&self, frequency: f32) -> f32 {
        let frequencies = &self.frequencies_hz[..self.point_count as usize];
        let gains = &self.gains_db[..self.point_count as usize];
        let frequency = frequency.max(1.0);
        if frequency <= frequencies[0] {
            return gains[0];
        }
        if frequency >= *frequencies.last().unwrap() {
            return *gains.last().unwrap();
        }
        for index in 0..frequencies.len() - 1 {
            if frequency <= frequencies[index + 1] {
                let low = frequencies[index].ln();
                let high = frequencies[index + 1].ln();
                if high <= low {
                    continue;
                }
                let amount = (frequency.ln() - low) / (high - low);
                return gains[index] + (gains[index + 1] - gains[index]) * amount;
            }
        }
        *gains.last().unwrap()
    }

    fn same_filter(&self, other: &Self) -> bool {
        let points = self.point_count as usize;
        self.preset == other.preset
            && self.base_preset == other.base_preset
            && self.point_count == other.point_count
            && self.frequencies_hz[..points] == other.frequencies_hz[..points]
            && self.gains_db[..points] == other.gains_db[..points]
    }
}

#[derive(Clone)]
struct EqImpulse([[Vec<f32>; 2]; 2]);

impl EqImpulse {
    fn for_preset(preset: EqPreset) -> Self {
        Self(match preset {
            EqPreset::Dialogue => load_zeroed_diagnol_hrtf!("eq/dialogue"),
            EqPreset::Acoustic => load_zeroed_diagnol_hrtf!("eq/acoustic"),
            EqPreset::BassBoost => load_zeroed_diagnol_hrtf!("eq/bassboost"),
            EqPreset::Orchestral => load_zeroed_diagnol_hrtf!("eq/orchestral"),
            EqPreset::Party => load_zeroed_diagnol_hrtf!("eq/party"),
            EqPreset::Electronic => load_zeroed_diagnol_hrtf!("eq/electronic"),
            EqPreset::Gaming => load_zeroed_diagnol_hrtf!("eq/gaming"),
            EqPreset::HipHop => load_zeroed_diagnol_hrtf!("eq/hiphop"),
            EqPreset::House => load_zeroed_diagnol_hrtf!("eq/house"),
            EqPreset::Jazz => load_zeroed_diagnol_hrtf!("eq/jazz"),
            EqPreset::Cinema => load_zeroed_diagnol_hrtf!("eq/cinema"),
            EqPreset::Pop => load_zeroed_diagnol_hrtf!("eq/pop"),
            EqPreset::Rock => load_zeroed_diagnol_hrtf!("eq/rock"),
            EqPreset::TrebleBoost => load_zeroed_diagnol_hrtf!("eq/trebleboost"),
            EqPreset::Off | EqPreset::Custom => unreachable!("preset has no measured response"),
        })
    }

    fn identity(length: usize) -> Self {
        let mut left = vec![0.0; length.max(1)];
        left[0] = 1.0;
        let right = left.clone();
        let zero = vec![0.0; length.max(1)];
        Self([[left, zero.clone()], [zero, right]])
    }

    fn length(&self) -> usize {
        self.0[0][0].len()
    }

    fn into_inner(self) -> [[Vec<f32>; 2]; 2] {
        self.0
    }

    fn apply_profile(&mut self, profile: &EqProfile) {
        for impulse in self.0.iter_mut().flatten() {
            if impulse.iter().all(|sample| *sample == 0.0) {
                continue;
            }
            let fft_size = impulse.len().next_power_of_two();
            let mut spectrum = vec![Complex::default(); fft_size];
            for (bin, sample) in spectrum.iter_mut().zip(impulse.iter()) {
                *bin = Complex::new(*sample, 0.0);
            }
            let mut transform = FourierTransform::new(fft_size);
            transform.forward(&mut spectrum);
            for (index, bin) in spectrum.iter_mut().enumerate() {
                let mirrored = index.min(fft_size - index);
                let frequency = mirrored as f32 * SAMPLE_RATE as f32 / fft_size as f32;
                *bin *= 10.0_f32.powf(profile.gain_at(frequency.max(1.0)) / 20.0);
            }
            transform.inverse(&mut spectrum);
            for (sample, bin) in impulse.iter_mut().zip(spectrum) {
                *sample = bin.re;
            }
        }
    }

    fn response_db(&self, frequencies: &[f32]) -> Vec<f32> {
        frequencies
            .iter()
            .map(|frequency| {
                let left = Self::magnitude(&self.0[0][0], *frequency);
                let right = Self::magnitude(&self.0[1][1], *frequency);
                20.0 * ((left + right) * 0.5).max(1.0e-6).log10()
            })
            .collect()
    }

    fn magnitude(impulse: &[f32], frequency: f32) -> f32 {
        let radians = -std::f32::consts::TAU * frequency / SAMPLE_RATE as f32;
        let (mut real, mut imaginary) = (0.0, 0.0);
        for (index, sample) in impulse.iter().enumerate() {
            let (sin, cos) = (radians * index as f32).sin_cos();
            real += sample * cos;
            imaginary += sample * sin;
        }
        real.hypot(imaginary)
    }
}

struct PresetFilter {
    impulses: EqImpulse,
    fir: Box<StereoFir>,
}

impl PresetFilter {
    fn new(preset: EqPreset) -> Self {
        let impulses = EqImpulse::for_preset(preset);
        Self {
            fir: StereoFir::new(impulses.clone().into_inner()),
            impulses,
        }
    }
}

pub struct EqEngine {
    presets: [PresetFilter; 14],
    // TODO: Add saves and loaf for custom allowing more than one custom.
    custom: Option<Box<StereoFir>>,
    active_profile: EqProfile,
}

impl EqEngine {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            presets: EqPreset::FILTERS.map(PresetFilter::new),
            custom: None,
            active_profile: EqProfile::default(),
        })
    }

    pub fn configure(&mut self, profile: EqProfile) {
        let profile = profile.sanitized();
        if profile == self.active_profile {
            return;
        }
        if profile.same_filter(&self.active_profile) {
            self.active_profile = profile;
            return;
        }
        if profile.preset == EqPreset::Custom {
            let mut impulses = profile
                .base_preset
                .filter_index()
                .map(|index| self.presets[index].impulses.clone())
                .unwrap_or_else(|| EqImpulse::identity(self.presets[0].impulses.length()));
            impulses.apply_profile(&profile);
            self.custom = Some(StereoFir::new(impulses.into_inner()));
        }
        self.active_profile = profile;
        self.reset();
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let preset = self.active_profile.preset;
        if let Some(index) = preset.filter_index() {
            self.presets[index].fir.process(left, right)
        } else if preset == EqPreset::Custom {
            self.custom
                .as_mut()
                .map_or((left, right), |filter| filter.process(left, right))
        } else {
            (left, right)
        }
    }

    pub fn reset(&mut self) {
        for preset in &mut self.presets {
            preset.fir.reset();
        }
        if let Some(custom) = &mut self.custom {
            custom.reset();
        }
    }
}
