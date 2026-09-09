mod server;

use serde::{Deserialize, Serialize};
pub use server::*;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Request {
    GetState,
    SetVolume(f32),
    SetMode(u8),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Response {
    State { volume: f32, mode: u8 },
    Ok,
    Error,
}
