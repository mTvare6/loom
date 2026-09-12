// SPDX-License-Identifier: MPL-2.0

use crate::convolution::{StereoFir, read_float_wave};

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/spatial-surround/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/spatial-surround/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/spatial-surround/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/spatial-surround/direct-right.wav");

/// A linear approximation of deconvolved as it wasn't completely time invariant
pub struct SpatialSurroundEngine {
    fir: Box<StereoFir>,
}

impl SpatialSurroundEngine {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            fir: StereoFir::new([
                [read_float_wave(DIRECT_LEFT), read_float_wave(RIGHT_TO_LEFT)],
                [
                    read_float_wave(LEFT_TO_RIGHT),
                    read_float_wave(DIRECT_RIGHT),
                ],
            ]),
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
