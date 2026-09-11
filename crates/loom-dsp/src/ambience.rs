use crate::convolution::{StereoFir, read_float_wave};

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/ambience/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/ambience/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/ambience/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/ambience/direct-right.wav");

pub struct AmbienceEngine {
    fir: Box<StereoFir>,
}

impl AmbienceEngine {
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
