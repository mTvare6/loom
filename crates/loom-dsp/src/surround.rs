// SPDX-License-Identifier: MPL-2.0

use crate::convolution::{StereoFir, read_float_wave};

// The extracted files are quieter because they were recorded at 46% volume
const LOUDNESS_GAIN: f32 = 2.186_642_4;

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/surround/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/surround/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/surround/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/surround/direct-right.wav");

pub struct SurroundEngine {
    fir: Box<StereoFir>,
}

impl SurroundEngine {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            fir: StereoFir::new([
                [read_float_wave(DIRECT_LEFT), read_float_wave(RIGHT_TO_LEFT)],
                [
                    read_float_wave(LEFT_TO_RIGHT),
                    read_float_wave(DIRECT_RIGHT),
                ],
            ]),
        })
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let (left, right) = self.fir.process(left, right);
        (left * LOUDNESS_GAIN, right * LOUDNESS_GAIN)
    }

    pub fn reset(&mut self) {
        self.fir.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const BLOCK_SIZE: usize = 512;

    #[test]
    fn stereo_impulse_preserves_response_shape_with_calibrated_gain() {
        let expected_left = read_float_wave(DIRECT_LEFT);
        let expected_right = read_float_wave(LEFT_TO_RIGHT);
        let mut engine = SurroundEngine::new();
        let mut output = Vec::with_capacity(BLOCK_SIZE + expected_left.len());
        for index in 0..BLOCK_SIZE + expected_left.len() {
            output.push(engine.process((index == 0) as u8 as f32, 0.0));
        }
        for index in 0..expected_left.len() {
            let actual = output[index + BLOCK_SIZE];
            assert!((actual.0 - expected_left[index] * LOUDNESS_GAIN).abs() < 4.0e-4);
            assert!((actual.1 - expected_right[index] * LOUDNESS_GAIN).abs() < 4.0e-4);
        }
    }
}
