use std::f32::consts::{PI, SQRT_2};

#[derive(Default)]
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
    pub fn new() -> Self {
        Self::default()
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

    pub fn set_low_shelf(&mut self, sample_rate: f32, freq: f32, gain_db: f32) {
        if gain_db.abs() < 0.01 {
            self.reset();
            return;
        }
        let a = f32::powf(10.0, gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let alpha = w0.sin() / 2.0 * SQRT_2;

        let a0 = (a + 1.0) + (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha;
        self.b0 = (a * ((a + 1.0) - (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha)) / a0;
        self.b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * w0.cos())) / a0;
        self.b2 = (a * ((a + 1.0) - (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha)) / a0;
        self.a1 = (-2.0 * ((a - 1.0) + (a + 1.0) * w0.cos())) / a0;
        self.a2 = ((a + 1.0) + (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha) / a0;
    }

    pub fn set_high_shelf(&mut self, sample_rate: f32, freq: f32, gain_db: f32) {
        if gain_db.abs() < 0.01 {
            self.reset();
            return;
        }
        let a = f32::powf(10.0, gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let alpha = w0.sin() / 2.0 * SQRT_2;

        let a0 = (a + 1.0) - (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha;
        self.b0 = (a * ((a + 1.0) + (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha)) / a0;
        self.b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * w0.cos())) / a0;
        self.b2 = (a * ((a + 1.0) + (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha)) / a0;
        self.a1 = (2.0 * ((a - 1.0) - (a + 1.0) * w0.cos())) / a0;
        self.a2 = ((a + 1.0) - (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha) / a0;
    }

    fn reset(&mut self) {
        self.b0 = 1.0;
        self.b1 = 0.0;
        self.b2 = 0.0;
        self.a1 = 0.0;
        self.a2 = 0.0;
    }
}

pub struct LoomEngine {
    delay_buffer: Vec<f32>,
    write_idx: usize,
    read_idx: usize,
    bass_eq_l: Biquad,
    bass_eq_r: Biquad,
    air_eq_l: Biquad,
    air_eq_r: Biquad,
    intensity: f32,
}

impl LoomEngine {
    pub fn new(sample_rate: f32) -> Self {
        // 15ms delay for Haas effect
        let delay_samples = ((15.0 / 1000.0) * sample_rate).round() as usize;
        let buffer_size = delay_samples.next_power_of_two().max(1024);

        Self {
            delay_buffer: vec![0.0; buffer_size],
            write_idx: delay_samples,
            read_idx: 0,
            bass_eq_l: Biquad::new(),
            bass_eq_r: Biquad::new(),
            air_eq_l: Biquad::new(),
            air_eq_r: Biquad::new(),
            intensity: 0.0,
        }
    }

    pub fn update_params(&mut self, intensity: f32, bass_db: f32) {
        self.intensity = intensity; // 0.0 to 1.0

        let air_db = intensity * 4.5;

        // Boost below 110Hz
        self.bass_eq_l.set_low_shelf(48000.0, 110.0, bass_db);
        self.bass_eq_r.set_low_shelf(48000.0, 110.0, bass_db);

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
            let l = self.air_eq_l.process(self.bass_eq_l.process(in_l));
            let r = self.air_eq_r.process(self.bass_eq_r.process(in_r));
            return (l, r);
        }

        // https://en.wikipedia.org/wiki/Joint_stereo#M/S_stereo_coding
        let mut mid = (in_l + in_r) * 0.5;
        let mut side = (in_l - in_r) * 0.5;

        // Low frequencies are non directional due to wavelengths longer
        // than the size of head mono reduces cancellation and phase going bad
        // https://en.wikipedia.org/wiki/Sound_localization
        // https://en.wikipedia.org/wiki/Psychoacoustics
        mid = self.bass_eq_l.process(mid);

        // High-frequencies attenuations are used to intuit angle and distance
        // Increasing the air part more on the side makes it feel wider
        // https://en.wikipedia.org/wiki/Psychoacoustics#Sound_localization
        side = self.air_eq_r.process(side);

        let width_boost = 1.0 + (self.intensity * 2.0);
        side *= width_boost;

        // https://en.wikipedia.org/wiki/Precedence_effect
        let delayed_side = self.delay_buffer[self.read_idx];
        self.delay_buffer[self.write_idx] = side;

        let mask = self.delay_buffer.len() - 1;
        self.write_idx = (self.write_idx + 1) & mask;
        self.read_idx = (self.read_idx + 1) & mask;

        // Terminology: DRY = current
        // Mixes dry (more dom) with delayed increasing the spatial tendency
        let processed_side = (side * 0.7) + (delayed_side * self.intensity * 0.5);

        let mut out_l = mid + processed_side;
        let mut out_r = mid - processed_side;

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
        out_l = self.air_eq_l.process(self.bass_eq_l.process(out_l));
        out_r = self.air_eq_r.process(self.bass_eq_r.process(out_r));

        (out_l, out_r)
    }
}
