use crate::delay::DelayLine;
use std::f32::consts::PI;

// Brain uses the delay between enchoes to intuit room size and angle of audio
// https://en.wikipedia.org/wiki/Precedence_effect
#[derive(Copy, Clone)]
struct ERTap {
    samples: usize,
    gain: f32,
    alpha: f32,
    state: f32,
}

pub(crate) struct EarlyReflections {
    delay: DelayLine<4096>,
    taps: [ERTap; 6],
}

impl EarlyReflections {
    pub(crate) fn new(sr: f32, configs: &[(f32, f32, f32)]) -> Self {
        let mut taps = [ERTap {
            samples: 0,
            gain: 0.0,
            alpha: 1.0,
            state: 0.0,
        }; 6];
        for (i, &(ms, g, c)) in configs.iter().enumerate() {
            let dt = 1.0 / sr;
            let rc = 1.0 / (2.0 * PI * c);
            taps[i] = ERTap {
                samples: (ms * sr / 1000.0) as usize,
                gain: g,
                alpha: dt / (rc + dt),
                state: 0.0,
            };
        }
        Self {
            delay: DelayLine::new(),
            taps,
        }
    }

    #[inline(always)]
    pub(crate) fn process(&mut self, x: f32) -> f32 {
        self.delay.buffer[self.delay.write_idx] = x;
        let mut out = 0.0;
        for tap in self.taps.iter_mut() {
            let read_idx = self.delay.write_idx.wrapping_sub(tap.samples) & 4095;
            // Wall damping on later bounces and air damping too
            tap.state = tap.state + tap.alpha * (self.delay.buffer[read_idx] - tap.state);
            out += tap.state * tap.gain;
        }
        self.delay.write_idx = (self.delay.write_idx + 1) & 4095;
        out
    }
}
