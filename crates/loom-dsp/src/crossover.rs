// SPDX-License-Identifier: MPL-2.0

use crate::Biquad;

// https://en.wikipedia.org/wiki/Linkwitz%E2%80%93Riley_filter
pub(crate) struct LR4 {
    lp1: Biquad,
    lp2: Biquad,
    hp1: Biquad,
    hp2: Biquad,
}

impl LR4 {
    pub(crate) fn new(sr: f32, freq: f32) -> Self {
        let mut lr = Self {
            lp1: Biquad::new(),
            lp2: Biquad::new(),
            hp1: Biquad::new(),
            hp2: Biquad::new(),
        };

        lr.lp1.set_lpf(sr, freq, 0.707);
        lr.lp2.set_lpf(sr, freq, 0.707);
        lr.hp1.set_hpf(sr, freq, 0.707);
        lr.hp2.set_hpf(sr, freq, 0.707);
        lr
    }

    #[inline(always)]
    pub(crate) fn process(&mut self, x: f32) -> (f32, f32) {
        (
            self.lp2.process(self.lp1.process(x)),
            self.hp2.process(self.hp1.process(x)),
        )
    }

    pub(crate) fn reset(&mut self) {
        self.lp1.reset();
        self.lp2.reset();
        self.hp1.reset();
        self.hp2.reset();
    }
}
