use crate::convolution::{StereoFir, read_float_wave};

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/spatial-stereo/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/spatial-stereo/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/spatial-stereo/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/spatial-stereo/direct-right.wav");

/// A linear approximation of deconvolved as it wasn't completely time invariant
pub struct SpatialStereoEngine {
    fir: Box<StereoFir>,
}

impl SpatialStereoEngine {
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
}
