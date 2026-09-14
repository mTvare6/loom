mod client;
mod server;

pub use client::*;
use serde::{Deserialize, Serialize};
pub use server::*;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Request {
    GetState,
    SetVolume(f32),
    SetMode(u8),
    SetPitchEnabled(bool),
    SetPitch(f32),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Response {
    State {
        volume: f32,
        mode: u8,
        pitch_enabled: bool,
        pitch: f32,
    },
    Ok,
    Error,
}
