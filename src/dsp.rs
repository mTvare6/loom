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
}

pub struct LoomEngine {
    cross1_l: LR4,
    cross1_r: LR4,
    haas_delay: DelayLine<1024>,
    delay_samples: usize,
    air_eq_l: Biquad,
    air_eq_r: Biquad,
    intensity: f32,
}

impl LoomEngine {
    pub fn new(sample_rate: f32) -> Self {
        let delay_samples = ((15.0 / 1000.0) * sample_rate).round() as usize;

        Self {
            cross1_l: LR4::new(sample_rate, 120.0),
            cross1_r: LR4::new(sample_rate, 120.0),
            haas_delay: DelayLine::new(),
            delay_samples,
            air_eq_l: Biquad::new(),
            air_eq_r: Biquad::new(),
            intensity: 0.0,
        }
    }

    pub fn update_params(&mut self, intensity: f32) {
        self.intensity = intensity;

        let air_db = intensity * 4.5;

        // Boost above 10kHz
        self.air_eq_l.set_high_shelf(48000.0, 10000.0, air_db);
        self.air_eq_r.set_high_shelf(48000.0, 10000.0, air_db);
    }

    #[inline(always)]
    // https://www.elementary.audio/docs/tutorials/distortion-saturation-wave-shaping
    fn parallel_exciter(x: f32, drive: f32) -> f32 {
        let wet = (x * drive).tanh();
        (x * 0.7) + (wet * 0.3)
    }

    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.intensity <= 0.01 {
            let l = self.air_eq_l.process(in_l);
            let r = self.air_eq_r.process(in_r);
            return (l, r);
        }

        let (low_l, high_l) = self.cross1_l.process(in_l);
        let (low_r, high_r) = self.cross1_r.process(in_r);

        // Low frequencies are non directional due to wavelengths longer
        // than the size of head mono reduces cancellation and phase going bad
        // https://en.wikipedia.org/wiki/Sound_localization
        // https://en.wikipedia.org/wiki/Psychoacoustics
        let out_low = (low_l + low_r) * 0.5 * (1.0 + self.intensity * 0.6);

        // https://en.wikipedia.org/wiki/Joint_stereo#M/S_stereo_coding
        let mid = (high_l + high_r) * 0.5;
        let mut side = (high_l - high_r) * 0.5;

        // High-frequencies attenuations are used to intuit angle and distance
        // Increasing the air part more on the side makes it feel wider
        // https://en.wikipedia.org/wiki/Psychoacoustics#Sound_localization
        side = self.air_eq_r.process(side);

        let width_boost = 1.0 + (self.intensity * 2.0);
        side *= width_boost;

        // https://en.wikipedia.org/wiki/Precedence_effect
        let delayed_side = self.haas_delay.process(side, self.delay_samples);

        // Terminology: DRY = current
        // Mixes dry (more dom) with delayed increasing the spatial tendency
        let processed_side = (side * 0.7) + (delayed_side * self.intensity * 0.5);

        let mut out_l = mid + processed_side;
        let mut out_r = mid - processed_side;

        // Add the dynamically scaled mono bass back in
        out_l += out_low;
        out_r += out_low;

        // More 2nd and 3rd order harmonics
        // Artificially reconstructs a deeper fundamental bass frequency when heard by brain
        // https://en.wikipedia.org/wiki/Missing_fundamental
        // https://www.soundonsound.com/techniques/all-about-exciters-enhancers
        // https://www.elementary.audio/docs/tutorials/distortion-saturation-wave-shaping
        // https://dsp.stackexchange.com/questions/17526/how-to-model-tape-saturation-audio-dsp
        // https://mural.maynoothuniversity.ie/id/eprint/4099/1/EAApaper-JT-30-03.pdf
        let drive = 1.0 + (self.intensity * 4.0);
        out_l = Self::parallel_exciter(out_l, drive);
        out_r = Self::parallel_exciter(out_r, drive);

        // Hearing at extreme lows and highs is bad, too bad at lower volumes
        // Dynamically compensate for it
        // https://en.wikipedia.org/wiki/Equal-loudness_contour
        // Smile EQ
        // \  -     -  /
        //  \         /
        //   \_______/
        // 100Hz   10kHz
        out_l = self.air_eq_l.process(out_l);
        out_r = self.air_eq_r.process(out_r);

        (out_l, out_r)
    }
}
