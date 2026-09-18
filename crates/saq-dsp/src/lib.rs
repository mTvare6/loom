// SPDX-License-Identifier: MPL-2.0

macro_rules! load_response_hrtf {
    ($directory:literal) => {
        [
            [
                $crate::convolution::read_float_wave(include_bytes!(concat!(
                    "../assets/",
                    $directory,
                    "/H_LL.wav"
                ))),
                $crate::convolution::read_float_wave(include_bytes!(concat!(
                    "../assets/",
                    $directory,
                    "/H_LR.wav"
                ))),
            ],
            [
                $crate::convolution::read_float_wave(include_bytes!(concat!(
                    "../assets/",
                    $directory,
                    "/H_RL.wav"
                ))),
                $crate::convolution::read_float_wave(include_bytes!(concat!(
                    "../assets/",
                    $directory,
                    "/H_RR.wav"
                ))),
            ],
        ]
    };
}

macro_rules! load_zeroed_diagnol_hrtf {
    ($directory:literal) => {{
        let h_ll = $crate::convolution::read_float_wave(include_bytes!(concat!(
            "../assets/",
            $directory,
            "/H_LL.wav"
        )));
        let zero = vec![0.0; h_ll.len()];
        [
            [h_ll, zero.clone()],
            [
                zero,
                $crate::convolution::read_float_wave(include_bytes!(concat!(
                    "../assets/",
                    $directory,
                    "/H_RR.wav"
                ))),
            ],
        ]
    }};
}

mod room;
mod biquad;
mod convolution;
mod crossfeed;
mod crossover;
mod decorrelator;
mod delay;
mod early_reflections;
mod fft;
mod clarity;
mod itd;
mod mode;
mod night;
mod pitch;
mod reverb;
mod spatial_filter;
mod spatial_stereo;
mod spatial_surround;
mod surround;

pub use room::RoomEngine;
pub use biquad::Biquad;
pub use clarity::ClarityEngine;
pub use mode::Mode;
pub use night::NightEngine;
pub use pitch::PitchEngine;
pub use spatial_filter::SpatialFilterEngine;
pub use spatial_stereo::SpatialStereoEngine;
pub use spatial_surround::SpatialSurroundEngine;
pub use surround::SurroundEngine;
