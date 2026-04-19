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

// https://en.wikipedia.org/wiki/Linkwitz%E2%80%93Riley_filter
struct LR4 {
    lp1: Biquad,
    lp2: Biquad,
    hp1: Biquad,
    hp2: Biquad,
}

impl LR4 {
    fn new(sr: f32, freq: f32) -> Self {
        let mut lr = Self {
            lp1: Biquad::new(),
            lp2: Biquad::new(),
            hp1: Biquad::new(),
            hp2: Biquad::new(),
        };

        lr.lp1.set_lpf(sr, freq, 0.707);
        lr.lp2.set_lpf(sr, freq, 0.707);
        lr.hp1.set_hpf(sr, freq, 0.707);
        lr.hp2.set_hpf(sr, freq, 0.707);
        lr
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> (f32, f32) {
        (
            self.lp2.process(self.lp1.process(x)),
            self.hp2.process(self.hp1.process(x)),
        )
    }
}

// N should be 2 power for the mask N - 1 to work
struct DelayLine<const N: usize> {
    buffer: [f32; N],
    write_idx: usize,
}

impl<const N: usize> DelayLine<N> {
    const fn new() -> Self {
        Self {
            buffer: [0.0; N],
            write_idx: 0,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32, delay_samples: usize) -> f32 {
        self.buffer[self.write_idx] = x;
        let read_idx = self.write_idx.wrapping_sub(delay_samples) & (N - 1);
        let out = self.buffer[read_idx];
        self.write_idx = (self.write_idx + 1) & (N - 1);
        out
    }

    // Fractional interpolation for between samples generated in steps like LFO
    // instead of rounded version approximations
    #[inline(always)]
    fn process_frac(&mut self, x: f32, delay_samples: f32) -> f32 {
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

// Tiny delay offset for the center channel
// https://en.wikipedia.org/wiki/Sound_localization#Duplex_theory
struct MicroITD {
    delay: DelayLine<128>,
    delay_samples: f32,
}

impl MicroITD {
    fn new(sr: f32, ms: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            delay_samples: ms * sr / 1000.0,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        self.delay.process_frac(x, self.delay_samples)
    }
}

// https://en.wikipedia.org/wiki/Acoustic_shadow
// Head blocks high freq from going to opp side
struct Crossfeed {
    delay: DelayLine<1024>,
    samples: usize,
    shelf: Biquad,
}

impl Crossfeed {
    fn new(sr: f32, delay_ms: f32, cutoff: f32) -> Self {
        let mut shelf = Biquad::new();
        shelf.set_high_shelf(sr, cutoff, -12.0);
        Self {
            delay: DelayLine::new(),
            samples: (delay_ms * sr / 1000.0) as usize,
            shelf,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        self.shelf.process(self.delay.process(x, self.samples))
    }
}

pub struct LoomEngine {
    cross1_l: LR4,
    cross1_r: LR4,
    cross2_l: LR4,
    cross2_r: LR4,

    pinna_notch: Biquad,
    center_itd: MicroITD,

    xf_l: Crossfeed,
    xf_r: Crossfeed,

    intensity: f32,
}

impl LoomEngine {
    pub fn new(sr: f32) -> Box<Self> {
        let mut pinna_notch = Biquad::new();

        // NOTE: Brings audible difference to non musical tones like speech/footsteps/game audio
        // by bringing it more forward
        pinna_notch.set_peaking(sr, 7500.0, 1.2, -6.0);

        Box::new(Self {
            cross1_l: LR4::new(sr, 120.0),
            cross1_r: LR4::new(sr, 120.0),
            cross2_l: LR4::new(sr, 4000.0),
            cross2_r: LR4::new(sr, 4000.0),

            pinna_notch,
            center_itd: MicroITD::new(sr, 0.15),

            xf_l: Crossfeed::new(sr, 0.25, 700.0),
            xf_r: Crossfeed::new(sr, 0.25, 700.0),

            intensity: 0.0,
        })
    }

    pub fn update_params(&mut self, intensity: f32) {
        self.intensity = intensity;
    }

    // Saturates nicely instead of harsh hollow feeling
    // https://www.elementary.audio/docs/tutorials/distortion-saturation-wave-shaping
    #[inline(always)]
    fn limit_side(x: f32) -> f32 {
        let x_clamp = x.clamp(-1.5, 1.5);
        x_clamp - (x_clamp * x_clamp * x_clamp) / 3.0
    }

    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.intensity <= 0.01 {
            return (in_l, in_r);
        }

        let (low_l, midhigh_l) = self.cross1_l.process(in_l);
        let (low_r, midhigh_r) = self.cross1_r.process(in_r);
        let (mid_l, high_l) = self.cross2_l.process(midhigh_l);
        let (mid_r, high_r) = self.cross2_r.process(midhigh_r);

        // Low frequencies are non directional due to wavelengths longer
        // than the size of head mono reduces cancellation and phase going bad
        let out_low = (low_l + low_r) * 0.5 * (1.0 + self.intensity * 0.6);

        // Head shadowing crossfeed
        let out_mid_l = mid_l + (self.xf_r.process(mid_r) * self.intensity * 0.4);
        let out_mid_r = mid_r + (self.xf_l.process(mid_l) * self.intensity * 0.4);

        let mid = (high_l + high_r) * 0.5;
        let side = (high_l - high_r) * 0.5;

        // Brings audible difference to non musical tones like speech footsteps game audio
        // by bringing it more forward
        let mid_eq = self.pinna_notch.process(mid);
        let mid_eq_l = self.center_itd.process(mid_eq);
        let mid_eq_r = mid_eq;

        // Simple static widening for now (Decorrelator comes next)
        let width_gain = 1.0 + self.intensity * 1.5;
        let side_l = Self::limit_side(side * width_gain * (1.0 + self.intensity * 2.0));
        let side_r = Self::limit_side(-side * width_gain * (1.0 + self.intensity * 2.0));

        let out_high_l = mid_eq_l + side_l;
        let out_high_r = mid_eq_r - side_r;

        (
            out_low + out_mid_l + out_high_l,
            out_low + out_mid_r + out_high_r,
        )
    }
}
