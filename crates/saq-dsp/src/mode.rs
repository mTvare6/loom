#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Mode {
    #[default]
    Off = 0,
    Spatial = 1,
    SurroundSound = 2,
}

impl Mode {
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Spatial,
            2 => Self::SurroundSound,
            _ => Self::Off,
        }
    }
}
