use crate::{
    Biquad, crossfeed::Crossfeed, crossover::LR4, decorrelator::ModAllPass,
    early_reflections::EarlyReflections, itd::MicroITD, reverb::StereoRoom,
};

pub struct SpatialFilterEngine {
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

impl SpatialFilterEngine {
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

    pub fn reset(&mut self) {
        self.cross1_l.reset();
        self.cross1_r.reset();
        self.cross2_l.reset();
        self.cross2_r.reset();
        self.pinna_notch.reset();
        self.center_itd.reset();
        self.xf_l.reset();
        self.xf_r.reset();
        self.decorr_l.reset();
        self.decorr_r.reset();
        self.er_l.reset();
        self.er_r.reset();
        self.room.reset();
        self.transient_env = 0.0;
    }

    // Saturates nicely instead of harsh hollow feeling
    // https://en.wikipedia.org/wiki/Waveshaper
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
        // https://en.wikipedia.org/wiki/Distortion#Audio_distortion
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
