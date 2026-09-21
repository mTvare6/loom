// SPDX-License-Identifier: MPL-2.0

use std::fmt;

#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Mode {
    Off = 0,
    SpatialFilter = 1,
    #[default]
    #[cfg_attr(feature = "cli", value(name = "surround-3d"))]
    SurroundSound = 2,
    Room = 3,
    Clarity = 4,
    Night = 5,
    SpatialStereo = 6,
    SpatialSurround = 7,
}

impl Mode {
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::SpatialFilter,
            2 => Self::SurroundSound,
            3 => Self::Room,
            4 => Self::Clarity,
            5 => Self::Night,
            6 => Self::SpatialStereo,
            7 => Self::SpatialSurround,
            _ => Self::Off,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Off => "off",
            Self::SpatialFilter => "spatial-filter",
            Self::SurroundSound => "surround-3d",
            Self::Room => "room",
            Self::Clarity => "clarity",
            Self::Night => "night",
            Self::SpatialStereo => "spatial-stereo",
            Self::SpatialSurround => "spatial-surround",
        })
    }
}
