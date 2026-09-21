// SPDX-License-Identifier: MPL-2.0

use crate::SAMPLE_RATE;
use signalsmith_stretch::Stretch;

const CHANNELS: u32 = 2;
const TONALITY_LIMIT_HZ: f32 = 8_000.0;
const MAX_CHUNK_FRAMES: usize = 4_096;

pub struct PitchEngine {
    stretch: Stretch,
    interleaved_input: Box<[f32]>,
    interleaved_output: Box<[f32]>,
    semitones: f32,
}

impl PitchEngine {
    pub fn new() -> Box<Self> {
        let mut engine = Box::new(Self {
            stretch: Stretch::preset_default(CHANNELS, SAMPLE_RATE),
            interleaved_input: vec![0.0; MAX_CHUNK_FRAMES * CHANNELS as usize].into_boxed_slice(),
            interleaved_output: vec![0.0; MAX_CHUNK_FRAMES * CHANNELS as usize].into_boxed_slice(),
            semitones: 0.0,
        });
        engine
            .stretch
            .set_transpose_factor_semitones(0.0, Some(TONALITY_LIMIT_HZ / SAMPLE_RATE as f32));
        engine.stretch.process(
            &engine.interleaved_input[..],
            &mut engine.interleaved_output[..],
        );
        engine.stretch.reset();
        engine
    }

    pub fn reset(&mut self) {
        self.stretch.reset();
    }

    // TODO: Use [Fletcher-Munson curve](https://en.wikipedia.org/wiki/Equal-loudness_contour) 
    // to make low frequency waves normalized to sound as loud as no-shift to preserve perceived
    // loudness on the same sound.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], semitones: f32) {
        debug_assert_eq!(left.len(), right.len());
        let semitones = semitones.clamp(-12.0, 12.0);
        if semitones.to_bits() != self.semitones.to_bits() {
            // 12 semitone is an octave
            self.stretch.set_transpose_factor_semitones(
                semitones,
                Some(TONALITY_LIMIT_HZ / SAMPLE_RATE as f32),
            );
            self.semitones = semitones;
        }

        for start in (0..left.len()).step_by(MAX_CHUNK_FRAMES) {
            let frames = (left.len() - start).min(MAX_CHUNK_FRAMES);
            let samples = frames * CHANNELS as usize;
            for frame in 0..frames {
                self.interleaved_input[frame * 2] = left[start + frame];
                self.interleaved_input[frame * 2 + 1] = right[start + frame];
            }
            self.stretch.process(
                &self.interleaved_input[..samples],
                &mut self.interleaved_output[..samples],
            );
            for frame in 0..frames {
                left[start + frame] = self.interleaved_output[frame * 2];
                right[start + frame] = self.interleaved_output[frame * 2 + 1];
            }
        }
    }

    pub fn latency_frames(&self) -> usize {
        self.stretch.input_latency() + self.stretch.output_latency()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    // Every sample_rate window has one crossings
    // signal_length / F has one crossing on unit frequency
    // length/F * f = crossing
    fn dominant_frequency(signal: &[f32], sample_rate: f32) -> f32 {
        let crossings = signal
            .windows(2)
            .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
            .count();
        crossings as f32 * sample_rate / signal.len() as f32
    }

    // On monotone A4 becoming A5
    #[test]
    fn shifts_pitch_without_changing_frame_count_or_stereo_relation() {
        let mut engine = PitchEngine::new();
        let frames = SAMPLE_RATE as usize * 3;
        let mut left: Vec<_> = (0..frames)
            .map(|index| (TAU * 440.0 * index as f32 / SAMPLE_RATE as f32).sin() * 0.2)
            .collect();
        let mut right = left.clone();
        for start in (0..frames).step_by(512) {
            let end = (start + 512).min(frames);
            engine.process(&mut left[start..end], &mut right[start..end], 12.0);
        }

        assert_eq!(left.len(), frames);
        assert_eq!(right.len(), frames);
        let settled = engine.latency_frames() + SAMPLE_RATE as usize;
        let frequency = dominant_frequency(&left[settled..], SAMPLE_RATE as f32);
        assert!((frequency - 880.0).abs() < 3.0, "measured {frequency} Hz");
        let stereo_error = left[settled..]
            .iter()
            .zip(&right[settled..])
            .map(|(left, right)| (left - right).powi(2))
            .sum::<f32>()
            / (left.len() - settled) as f32;
        let stereo_error = stereo_error.sqrt();
        assert!(stereo_error < 1.0e-2, "stereo RMS error {stereo_error}");
    }
}
