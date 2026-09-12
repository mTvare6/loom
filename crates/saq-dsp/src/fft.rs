// SPDX-License-Identifier: MPL-2.0

use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::sync::Arc;

pub(crate) type Complex = Complex32;

pub(crate) struct FourierTransform {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    forward_scratch: Vec<Complex>,
    inverse_scratch: Vec<Complex>,
    inverse_scale: f32,
}

impl FourierTransform {
    pub(crate) fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(size);
        let inverse = planner.plan_fft_inverse(size);
        let forward_scratch = vec![Complex::default(); forward.get_inplace_scratch_len()];
        let inverse_scratch = vec![Complex::default(); inverse.get_inplace_scratch_len()];

        Self {
            forward,
            inverse,
            forward_scratch,
            inverse_scratch,
            inverse_scale: 1.0 / size as f32,
        }
    }

    #[inline]
    pub(crate) fn forward(&mut self, values: &mut [Complex]) {
        self.forward
            .process_with_scratch(values, &mut self.forward_scratch);
    }

    #[inline]
    pub(crate) fn inverse(&mut self, values: &mut [Complex]) {
        self.inverse
            .process_with_scratch(values, &mut self.inverse_scratch);
        for value in values {
            *value *= self.inverse_scale;
        }
    }
}
