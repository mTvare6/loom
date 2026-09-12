use crate::convolution::{StereoFir, read_float_wave};

const SAMPLE_RATE: f32 = 48_000.0;
const THRESHOLD_DBFS: f32 = -16.523_478;
const GAIN_REDUCTION_SLOPE: f32 = 0.757_800_04;
const ATTACK_SECONDS: f32 = 0.013_295_716;
const RELEASE_SECONDS: f32 = 0.115_254_22;

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/night/direct-left.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/night/direct-right.wav");

pub struct NightEngine {
    fir: Box<StereoFir>,
    detector_power: f32,
    attack_coefficient: f32,
    release_coefficient: f32,
}

impl NightEngine {
    pub fn new() -> Box<Self> {
        let zero = vec![0.0; read_float_wave(DIRECT_LEFT).len()];
        Box::new(Self {
            fir: StereoFir::new([
                [read_float_wave(DIRECT_LEFT), zero.clone()],
                [zero, read_float_wave(DIRECT_RIGHT)],
            ]),
            detector_power: 0.0,
            attack_coefficient: (-1.0 / (ATTACK_SECONDS * SAMPLE_RATE)).exp(),
            release_coefficient: (-1.0 / (RELEASE_SECONDS * SAMPLE_RATE)).exp(),
        })
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let (left, right) = self.fir.process(left, right);
        let mid = (left + right) * 0.5;
        let instantaneous_power = mid * mid;
        let coefficient = if instantaneous_power > self.detector_power {
            self.attack_coefficient
        } else {
            self.release_coefficient
        };
        self.detector_power =
            coefficient * self.detector_power + (1.0 - coefficient) * instantaneous_power;
        let detector_db = 10.0 * self.detector_power.max(1.0e-20).log10();
        // If louder than -16dB, gain by GAIN_REDUCTION_SLOPE level in dB
        let reduction_db = GAIN_REDUCTION_SLOPE * (detector_db - THRESHOLD_DBFS).max(0.0);
        let gain = 10.0_f32.powf(-reduction_db / 20.0);
        (left * gain, right * gain)
    }

    pub fn reset(&mut self) {
        self.fir.reset();
        self.detector_power = 0.0;
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Verify on another round of testing if this is intended
    // or the filter was broken and check other sources for similiar
    // description of intended behaviour for a night mode.
    #[test]
    fn anti_phase_signal_does_not_drive_linked_detector() {
        let mut anti_phase = NightEngine::new();
        let mut in_phase = NightEngine::new();
        for _ in 0..48_000 {
            anti_phase.process(0.5, -0.5);
            in_phase.process(0.5, 0.5);
        }
        assert!(anti_phase.detector_power < in_phase.detector_power * 1.0e-3);
    }
}
