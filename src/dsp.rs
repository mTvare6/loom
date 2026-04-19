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
    // instead of unded rounded version approximations
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

// Head blocks high freq from going to opp side
// https://en.wikipedia.org/wiki/Acoustic_shadow
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

// https://en.wikipedia.org/wiki/Decorrelation
// Breaks mono coherence by shifting phase
struct ModAllPass {
    delay: DelayLine<2048>,
    base_delay: f32,
    mod_depth: f32,
    coeff: f32,
    lfo_phase: f32,
    lfo_inc: f32,
}

impl ModAllPass {
    fn new(sr: f32, ms: f32, depth_ms: f32, rate_hz: f32, coeff: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            base_delay: ms * sr / 1000.0,
            mod_depth: depth_ms * sr / 1000.0,
            coeff,
            lfo_phase: 0.0,
            lfo_inc: 2.0 * PI * rate_hz / sr,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        let current_delay = self.base_delay + (self.lfo_phase.sin() * self.mod_depth);

        self.lfo_phase += self.lfo_inc;
        // HACK: to improve performance just substract instead of div
        if self.lfo_phase >= 2.0 * PI {
            self.lfo_phase -= 2.0 * PI;
        }

        let delayed = self.delay.process_frac(x, current_delay);
        let out = delayed - self.coeff * x;
        self.delay.buffer[(self.delay.write_idx.wrapping_sub(1)) & 2047] = x + self.coeff * delayed;
        out
    }
}

// Brain uses the delay between enchoes to intuit room size and angle of audio
// https://en.wikipedia.org/wiki/Precedence_effect
#[derive(Copy, Clone)]
struct ERTap {
    samples: usize,
    gain: f32,
    alpha: f32,
    state: f32,
}

struct EarlyReflections {
    delay: DelayLine<4096>,
    taps: [ERTap; 6],
}

impl EarlyReflections {
    fn new(sr: f32, configs: &[(f32, f32, f32)]) -> Self {
        let mut taps = [ERTap {
            samples: 0,
            gain: 0.0,
            alpha: 1.0,
            state: 0.0,
        }; 6];
        for (i, &(ms, g, c)) in configs.iter().enumerate() {
            let dt = 1.0 / sr;
            let rc = 1.0 / (2.0 * PI * c);
            taps[i] = ERTap {
                samples: (ms * sr / 1000.0) as usize,
                gain: g,
                alpha: dt / (rc + dt),
                state: 0.0,
            };
        }
        Self {
            delay: DelayLine::new(),
            taps,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        self.delay.buffer[self.delay.write_idx] = x;
        let mut out = 0.0;
        for tap in self.taps.iter_mut() {
            let read_idx = self.delay.write_idx.wrapping_sub(tap.samples) & 4095;
            // Wall damping on later bounces and air damping too
            tap.state = tap.state + tap.alpha * (self.delay.buffer[read_idx] - tap.state);
            out += tap.state * tap.gain;
        }
        self.delay.write_idx = (self.delay.write_idx + 1) & 4095;
        out
    }
}

// https://ccrma.stanford.edu/~jos/pasp/Schroeder_Reverberators.html
struct Comb<const N: usize> {
    delay: DelayLine<N>,
    samples: usize,
    feedback: f32,
    damp_state: f32,
}

impl<const N: usize> Comb<N> {
    fn new(sr: f32, ms: f32, feedback: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            samples: (ms * sr / 1000.0) as usize,
            feedback,
            damp_state: 0.0,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        let read_idx = self.delay.write_idx.wrapping_sub(self.samples) & (N - 1);
        let out = self.delay.buffer[read_idx];
        // Mild LPF in feedback loop
        self.damp_state = (out * 0.6) + (self.damp_state * 0.4);
        self.delay.buffer[self.delay.write_idx] = x + (self.damp_state * self.feedback);
        self.delay.write_idx = (self.delay.write_idx + 1) & (N - 1);
        out
    }
}

struct StaticAllpass<const N: usize> {
    delay: DelayLine<N>,
    samples: usize,
    coeff: f32,
}

impl<const N: usize> StaticAllpass<N> {
    fn new(sr: f32, ms: f32, coeff: f32) -> Self {
        Self {
            delay: DelayLine::new(),
            samples: (ms * sr / 1000.0) as usize,
            coeff,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        let read_idx = self.delay.write_idx.wrapping_sub(self.samples) & (N - 1);
        let delayed = self.delay.buffer[read_idx];
        let out = delayed - self.coeff * x;
        self.delay.buffer[self.delay.write_idx] = x + self.coeff * delayed;
        self.delay.write_idx = (self.delay.write_idx + 1) & (N - 1);
        out
    }
}

// Instead of the isolated reverb where L is lerped onto R this simulates energy mechanisms
// https://ccrma.stanford.edu/~jos/pasp/Feedback_Delay_Networks.html
struct StereoRoom {
    c_l1: Comb<4096>,
    c_l2: Comb<4096>,
    c_r1: Comb<4096>,
    c_r2: Comb<4096>,
    a_l1: StaticAllpass<1024>,
    a_r1: StaticAllpass<1024>,
}

impl StereoRoom {
    fn new(sr: f32) -> Self {
        Self {
            // Asymmetrical comb delays for L and R
            c_l1: Comb::new(sr, 29.3, 0.75),
            c_l2: Comb::new(sr, 37.1, 0.70),
            c_r1: Comb::new(sr, 31.7, 0.75),
            c_r2: Comb::new(sr, 41.9, 0.70),

            // Asymmetrical allpass diffusion
            a_l1: StaticAllpass::new(sr, 5.3, 0.5),
            a_r1: StaticAllpass::new(sr, 4.7, 0.5),
        }
    }

    #[inline(always)]
    fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Non normalised mix
        let mixed_l = in_l + (in_r * 0.3);
        let mixed_r = in_r + (in_l * 0.3);

        // Through parallel combs
        let combs_l = self.c_l1.process(mixed_l) + self.c_l2.process(mixed_l);
        let combs_r = self.c_r1.process(mixed_r) + self.c_r2.process(mixed_r);

        // Through series allpass for late diffusion
        let out_l = self.a_l1.process(combs_l * 0.5);
        let out_r = self.a_r1.process(combs_r * 0.5);

        (out_l, out_r)
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
    decorr_l: ModAllPass,
    decorr_r: ModAllPass,
    er_l: EarlyReflections,
    er_r: EarlyReflections,
    room: StereoRoom,

    transient_env: f32,
    attack_coef: f32,
    release_coef: f32,
    intensity: f32,
}

impl LoomEngine {
    pub fn new(sr: f32) -> Box<Self> {
        // Taps are delay ms then gain then air absorption LPF Hz
        let t_l = [
            (7.0, 0.45, 12000.0),
            (13.0, 0.35, 8000.0),
            (19.0, 0.25, 5000.0),
            (23.0, 0.20, 3000.0),
            (31.0, 0.15, 2000.0),
            (41.0, 0.10, 1000.0),
        ];
        let t_r = [
            (11.0, 0.45, 12000.0),
            (17.0, 0.35, 8000.0),
            (29.0, 0.25, 5000.0),
            (37.0, 0.20, 3000.0),
            (43.0, 0.15, 2000.0),
            (47.0, 0.10, 1000.0),
        ];

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

            // Dual decorrelators with out of phase LFOs for more natuural phase drift
            decorr_l: ModAllPass::new(sr, 2.5, 0.5, 0.15, 0.6),
            decorr_r: ModAllPass::new(sr, 3.1, 0.7, 0.11, 0.6),

            er_l: EarlyReflections::new(sr, &t_l),
            er_r: EarlyReflections::new(sr, &t_r),
            room: StereoRoom::new(sr),

            transient_env: 0.0,
            attack_coef: (-1.0 / (2.0 * 0.001 * sr)).exp(),
            release_coef: (-1.0 / (50.0 * 0.001 * sr)).exp(),
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

        // https://en.wikipedia.org/wiki/Envelope_detector
        // When a sharp sound happens reduce the side mult to not muddle
        let mid_mono = (mid_l + mid_r) * 0.5;
        let peak = mid_mono.abs();

        let coef = if peak > self.transient_env {
            self.attack_coef
        } else {
            self.release_coef
        };

        self.transient_env = coef * (self.transient_env - peak) + peak;
        let dynamic_width_mod = 1.0 - (self.transient_env * 2.0).clamp(0.0, 0.6);

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

        let mut side_l = self.decorr_l.process(side);
        let mut side_r = self.decorr_r.process(-side);

        // More 2nd and 3rd order harmonics
        // Makes the frequencies more audible and distinct
        // https://en.wikipedia.org/wiki/Missing_fundamental
        // https://www.soundonsound.com/techniques/all-about-exciters-enhancers
        // https://www.elementary.audio/docs/tutorials/distortion-saturation-wave-shaping
        let width_gain = (1.0 + self.intensity * 1.5) * dynamic_width_mod;
        side_l = Self::limit_side(side_l * width_gain * (1.0 + self.intensity * 2.0));
        side_r = Self::limit_side(side_r * width_gain * (1.0 + self.intensity * 2.0));

        let wide_high_l = mid_eq_l + side_l;
        let wide_high_r = mid_eq_r - side_r;

        let er_l = self.er_l.process(wide_high_l);
        let er_r = self.er_r.process(wide_high_r);

        // Early reflections for dense late tail
        let (tail_l, tail_r) = self.room.process(er_l, er_r);

        // Mixes dry more dom with delayed increasing the spatial tendency
        let out_high_l =
            wide_high_l + (er_l * self.intensity * 0.7) + (tail_l * self.intensity * 0.15);
        let out_high_r =
            wide_high_r + (er_r * self.intensity * 0.7) + (tail_r * self.intensity * 0.15);

        (
            out_low + out_mid_l + out_high_l,
            out_low + out_mid_r + out_high_r,
        )
    }
}
