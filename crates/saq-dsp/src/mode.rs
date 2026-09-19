// SPDX-License-Identifier: MPL-2.0

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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Off" => Some(Self::Off),
            "SpatialFilter" => Some(Self::SpatialFilter),
            "SurroundSound" => Some(Self::SurroundSound),
            "Room" => Some(Self::Room),
            "Clarity" => Some(Self::Clarity),
            "Night" => Some(Self::Night),
            "SpatialStereo" => Some(Self::SpatialStereo),
            "SpatialSurround" => Some(Self::SpatialSurround),
            _ => None,
        }
    }
}
