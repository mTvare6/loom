// SPDX-License-Identifier: MPL-2.0

use crate::convolution::{StereoFir, read_float_wave};

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/fidelity/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/fidelity/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/fidelity/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/fidelity/direct-right.wav");

pub struct FidelityEngine {
    fir: Box<StereoFir>,
}

impl FidelityEngine {
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
        self.fir.process(left, right)
    }

    pub fn reset(&mut self) {
        self.fir.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_output(left: f32, right: f32) -> (f32, f32) {
        let mut engine = FidelityEngine::new();
        engine.process(left, right);
        for _ in 1..512 {
            engine.process(0.0, 0.0);
        }
        engine.process(0.0, 0.0)
    }

    #[test]
    fn applies_all_four_measured_paths() {
        let from_left = first_output(1.0, 0.0);
        let from_right = first_output(0.0, 1.0);

        assert!((from_left.0 - read_float_wave(DIRECT_LEFT)[0]).abs() < 1e-6);
        assert!((from_left.1 - read_float_wave(LEFT_TO_RIGHT)[0]).abs() < 1e-6);
        assert!((from_right.0 - read_float_wave(RIGHT_TO_LEFT)[0]).abs() < 1e-6);
        assert!((from_right.1 - read_float_wave(DIRECT_RIGHT)[0]).abs() < 1e-6);
    }
}
