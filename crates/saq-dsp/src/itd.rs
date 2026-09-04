use crate::delay::DelayLine;

// Tiny delay offset for the center channel
// https://en.wikipedia.org/wiki/Sound_localization#Duplex_theory
pub(crate) struct MicroITD {
    delay: DelayLine<128>,
    delay_samples: f32,
}

impl MicroITD {
    pub(crate) fn new(sr: f32, ms: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            delay_samples: ms * sr / 1000.0,
        }
    }

    #[inline(always)]
    pub(crate) fn process(&mut self, x: f32) -> f32 {
        self.delay.process_frac(x, self.delay_samples)
    }
}
