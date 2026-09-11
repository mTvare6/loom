use crate::convolution::{StereoFir, read_float_wave};

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/room/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/room/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/room/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/room/direct-right.wav");

pub struct RoomEngine {
    fir: Box<StereoFir>,
}

impl RoomEngine {
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
