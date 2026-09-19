// SPDX-License-Identifier: MPL-2.0

use crate::convolution::StereoFir;

/// A linear approximation of deconvolved as it wasn't completely time invariant
pub struct SpatialStereoEngine {
    fir: Box<StereoFir>,
}

impl SpatialStereoEngine {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            fir: StereoFir::new(load_response_hrtf!("spatial-stereo")),
        })
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        self.fir.process(left, right)
    }

    pub fn reset(&mut self) {
        self.fir.reset();
    }
}
