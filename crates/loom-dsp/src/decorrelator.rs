use crate::delay::DelayLine;
use std::f32::consts::PI;

// https://en.wikipedia.org/wiki/Decorrelation
// Breaks mono coherence by shifting phase
pub(crate) struct ModAllPass {
    delay: DelayLine<2048>,
    base_delay: f32,
    mod_depth: f32,
    coeff: f32,
    lfo_phase: f32,
    lfo_inc: f32,
}

impl ModAllPass {
    pub(crate) fn new(sr: f32, ms: f32, depth_ms: f32, rate_hz: f32, coeff: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            base_delay: ms * sr / 1000.0,
            mod_depth: depth_ms * sr / 1000.0,
            coeff,
            lfo_phase: 0.0,
            lfo_inc: 2.0 * PI * rate_hz / sr,
        }
    }

    #[inline(always)]
    pub(crate) fn process(&mut self, x: f32) -> f32 {
        let current_delay = self.base_delay + (self.lfo_phase.sin() * self.mod_depth);

        self.lfo_phase += self.lfo_inc;
        // HACK: to improve performance just substract instead of div
        if self.lfo_phase >= 2.0 * PI {
            self.lfo_phase -= 2.0 * PI;
        }

        let delayed = self.delay.process_frac(x, current_delay);
        let out = delayed - self.coeff * x;
        self.delay.buffer[(self.delay.write_idx.wrapping_sub(1)) & 2047] = x + self.coeff * delayed;
        out
    }

    pub(crate) fn reset(&mut self) {
        self.delay.reset();
        self.lfo_phase = 0.0;
    }
}
