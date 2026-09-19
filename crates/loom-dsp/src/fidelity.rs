// SPDX-License-Identifier: MPL-2.0

use crate::convolution::StereoFir;

pub struct FidelityEngine {
    fir: Box<StereoFir>,
}

impl FidelityEngine {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            fir: StereoFir::new(load_response_hrtf!("fidelity")),
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
        let response = load_response_hrtf!("fidelity");

        assert!((from_left.0 - response[0][0][0]).abs() < 1e-6);
        assert!((from_left.1 - response[1][0][0]).abs() < 1e-6);
        assert!((from_right.0 - response[0][1][0]).abs() < 1e-6);
        assert!((from_right.1 - response[1][1][0]).abs() < 1e-6);
    }
}
