use crate::{Biquad, delay::DelayLine};

// Head blocks high freq from going to opp side
// https://en.wikipedia.org/wiki/Acoustic_shadow
pub(crate) struct Crossfeed {
    delay: DelayLine<1024>,
    samples: usize,
    shelf: Biquad,
}

impl Crossfeed {
    pub(crate) fn new(sr: f32, delay_ms: f32, cutoff: f32) -> Self {
        let mut shelf = Biquad::new();
        shelf.set_high_shelf(sr, cutoff, -12.0);
        Self {
            delay: DelayLine::new(),
            samples: (delay_ms * sr / 1000.0) as usize,
            shelf,
        }
    }

    #[inline(always)]
    pub(crate) fn process(&mut self, x: f32) -> f32 {
        self.shelf.process(self.delay.process(x, self.samples))
    }

    pub(crate) fn reset(&mut self) {
        self.delay.reset();
        self.shelf.reset();
    }
}
