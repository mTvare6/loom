// SPDX-License-Identifier: MPL-2.0

use crate::convolution::StereoFir;

pub struct AmbienceEngine {
    fir: Box<StereoFir>,
}

impl AmbienceEngine {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            fir: StereoFir::new(load_response_hrtf!("ambience")),
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
