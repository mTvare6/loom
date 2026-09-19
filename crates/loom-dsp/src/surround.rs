// SPDX-License-Identifier: MPL-2.0

use crate::convolution::InterpolatedStereoFir;

// H(s) = H_0 + g_1(s)*(H_100 - H_0) + g_2(s)*C
// TODO: Study and analyse response at more points
// g_1(s) at subdivided points
const PRIMARY_GAINS: [f32; 6] = [
    0.0,
    0.062_579_01,
    0.189_306_21,
    0.393_769_92,
    0.656_691_2,
    1.0,
];

// g_2(s) at subdivided points
const CORRECTION_GAINS: [f32; 6] = [0.0, 0.457_270_77, 0.860_252_14, 1.0, 0.761_640_85, 0.0];

pub struct SurroundEngine {
    fir: Box<InterpolatedStereoFir>,
}
impl SurroundEngine {
    pub fn new() -> Box<Self> {
        Self::new_at(1.0)
    }

    fn new_at(subwoofer: f32) -> Box<Self> {
        Box::new(Self {
            fir: InterpolatedStereoFir::new(
                load_response_hrtf!("surround/base"),
                [
                    (load_response_hrtf!("surround/branch"), PRIMARY_GAINS),
                    (load_response_hrtf!("surround/correction"), CORRECTION_GAINS),
                ],
                subwoofer,
            ),
        })
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32, subwoofer: f32) -> (f32, f32) {
        self.fir.process(left, right, subwoofer)
    }

    pub fn reset(&mut self) {
        self.fir.reset();
    }
}
