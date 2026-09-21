// SPDX-License-Identifier: MPL-2.0

use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Mode {
    Off = 0,
    SpatialFilter = 1,
    #[default]
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

impl FromStr for Mode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "off" => Ok(Self::Off),
            "spatial-filter" => Ok(Self::SpatialFilter),
            "surround-3d" => Ok(Self::SurroundSound),
            "room" => Ok(Self::Room),
            "clarity" => Ok(Self::Clarity),
            "night" => Ok(Self::Night),
            "spatial-stereo" => Ok(Self::SpatialStereo),
            "spatial-surround" => Ok(Self::SpatialSurround),
            _ => Err(format!("unknown mode '{value}'")),
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
