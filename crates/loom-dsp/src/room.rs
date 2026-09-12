// SPDX-License-Identifier: MPL-2.0

use crate::delay::DelayLine;

// https://ccrma.stanford.edu/~jos/pasp/Schroeder_Reverberators.html
struct Comb<const N: usize> {
    delay: DelayLine<N>,
    samples: usize,
    feedback: f32,
    damp_state: f32,
}

impl<const N: usize> Comb<N> {
    fn new(sr: f32, ms: f32, feedback: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            samples: (ms * sr / 1000.0) as usize,
            feedback,
            damp_state: 0.0,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        let read_idx = self.delay.write_idx.wrapping_sub(self.samples) & (N - 1);
        let out = self.delay.buffer[read_idx];
        // Mild LPF in feedback loop
        self.damp_state = (out * 0.6) + (self.damp_state * 0.4);
        self.delay.buffer[self.delay.write_idx] = x + (self.damp_state * self.feedback);
        self.delay.write_idx = (self.delay.write_idx + 1) & (N - 1);
        out
    }

    fn reset(&mut self) {
        self.delay.reset();
        self.damp_state = 0.0;
    }
}

struct StaticAllpass<const N: usize> {
    delay: DelayLine<N>,
    samples: usize,
    coeff: f32,
}

impl<const N: usize> StaticAllpass<N> {
    fn new(sr: f32, ms: f32, coeff: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            samples: (ms * sr / 1000.0) as usize,
            coeff,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        let read_idx = self.delay.write_idx.wrapping_sub(self.samples) & (N - 1);
        let delayed = self.delay.buffer[read_idx];
        let out = delayed - self.coeff * x;
        self.delay.buffer[self.delay.write_idx] = x + self.coeff * delayed;
        self.delay.write_idx = (self.delay.write_idx + 1) & (N - 1);
        out
    }

    fn reset(&mut self) {
        self.delay.reset();
    }
}

// Instead of the isolated reverb where L is lerped onto R this simulates energy mechanisms
// https://ccrma.stanford.edu/~jos/pasp/Feedback_Delay_Networks.html
pub(crate) struct StereoRoom {
    c_l1: Comb<4096>,
    c_l2: Comb<4096>,
    c_r1: Comb<4096>,
    c_r2: Comb<4096>,
    a_l1: StaticAllpass<1024>,
    a_r1: StaticAllpass<1024>,
}

impl StereoRoom {
    pub(crate) fn new(sr: f32) -> Self {
        Self {
            // Asymmetrical comb delays for L and R
            c_l1: Comb::new(sr, 29.3, 0.75),
            c_l2: Comb::new(sr, 37.1, 0.70),
            c_r1: Comb::new(sr, 31.7, 0.75),
            c_r2: Comb::new(sr, 41.9, 0.70),

            // Asymmetrical allpass diffusion
            a_l1: StaticAllpass::new(sr, 5.3, 0.5),
            a_r1: StaticAllpass::new(sr, 4.7, 0.5),
        }
    }

    #[inline(always)]
    pub(crate) fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Non normalised mix
        let mixed_l = in_l + (in_r * 0.3);
        let mixed_r = in_r + (in_l * 0.3);

        // Through parallel combs
        let combs_l = self.c_l1.process(mixed_l) + self.c_l2.process(mixed_l);
        let combs_r = self.c_r1.process(mixed_r) + self.c_r2.process(mixed_r);

        // Through series allpass for late diffusion
        let out_l = self.a_l1.process(combs_l * 0.5);
        let out_r = self.a_r1.process(combs_r * 0.5);

        (out_l, out_r)
    }

    pub(crate) fn reset(&mut self) {
        self.c_l1.reset();
        self.c_l2.reset();
        self.c_r1.reset();
        self.c_r2.reset();
        self.a_l1.reset();
        self.a_r1.reset();
    }
}
