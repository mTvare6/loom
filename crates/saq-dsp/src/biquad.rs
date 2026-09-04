use std::f32::consts::{PI, SQRT_2};

// https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html
#[derive(Clone, Copy, Default)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    pub const fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    pub fn set_lpf(&mut self, sr: f32, freq: f32, q: f32) {
        let w0 = 2.0 * PI * freq / sr;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 - w0.cos()) / 2.0) / a0;
        self.b1 = (1.0 - w0.cos()) / a0;
        self.b2 = ((1.0 - w0.cos()) / 2.0) / a0;
        self.a1 = (-2.0 * w0.cos()) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_hpf(&mut self, sr: f32, freq: f32, q: f32) {
        let w0 = 2.0 * PI * freq / sr;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + w0.cos()) / 2.0) / a0;
        self.b1 = -(1.0 + w0.cos()) / a0;
        self.b2 = ((1.0 + w0.cos()) / 2.0) / a0;
        self.a1 = (-2.0 * w0.cos()) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_peaking(&mut self, sr: f32, freq: f32, q: f32, gain_db: f32) {
        let a = f32::powf(10.0, gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sr;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * w0.cos()) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * w0.cos()) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }

    pub fn set_high_shelf(&mut self, sr: f32, freq: f32, gain_db: f32) {
        let a = f32::powf(10.0, gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sr;
        let alpha = w0.sin() / 2.0 * SQRT_2;
        let a0 = (a + 1.0) - (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha;
        self.b0 = (a * ((a + 1.0) + (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha)) / a0;
        self.b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * w0.cos())) / a0;
        self.b2 = (a * ((a + 1.0) + (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha)) / a0;
        self.a1 = (2.0 * ((a - 1.0) - (a + 1.0) * w0.cos())) / a0;
        self.a2 = ((a + 1.0) - (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha) / a0;
    }
}
