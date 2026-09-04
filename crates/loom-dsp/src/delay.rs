// N should be 2 power for the mask N - 1 to work
pub(crate) struct DelayLine<const N: usize> {
    pub(crate) buffer: [f32; N],
    pub(crate) write_idx: usize,
}

impl<const N: usize> DelayLine<N> {
    pub(crate) const fn new() -> Self {
        Self {
            buffer: [0.0; N],
            write_idx: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn process(&mut self, x: f32, delay_samples: usize) -> f32 {
        self.buffer[self.write_idx] = x;
        let read_idx = self.write_idx.wrapping_sub(delay_samples) & (N - 1);
        let out = self.buffer[read_idx];
        self.write_idx = (self.write_idx + 1) & (N - 1);
        out
    }

    // Fractional interpolation for between samples generated in steps like LFO
    // instead of unded rounded version approximations
    #[inline(always)]
    pub(crate) fn process_frac(&mut self, x: f32, delay_samples: f32) -> f32 {
        self.buffer[self.write_idx] = x;

        let d_int = delay_samples.trunc() as usize;
        let frac = delay_samples.fract();

        let idx1 = self.write_idx.wrapping_sub(d_int) & (N - 1);
        let idx2 = self.write_idx.wrapping_sub(d_int + 1) & (N - 1);

        let out = self.buffer[idx1] * (1.0 - frac) + self.buffer[idx2] * frac;

        self.write_idx = (self.write_idx + 1) & (N - 1);
        out
    }
}
