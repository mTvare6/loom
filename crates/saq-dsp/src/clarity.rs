use crate::convolution::{StereoFir, read_float_wave};

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/clarity/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/clarity/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/clarity/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/clarity/direct-right.wav");

pub struct ClarityEngine {
    fir: Box<StereoFir>,
}

impl ClarityEngine {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_output(left: f32, right: f32) -> (f32, f32) {
        let mut engine = ClarityEngine::new();
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
