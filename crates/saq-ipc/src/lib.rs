mod client;
mod server;

pub use client::*;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
pub use server::*;
use std::{
    io::{self, Error, ErrorKind},
    path::PathBuf,
};

pub fn socket_path() -> io::Result<PathBuf> {
    let directories = ProjectDirs::from("com", "epestr", "saq")
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Home directory is unavailable"))?;
    let runtime_directory = directories
        .runtime_dir()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "User runtime directory found"))?;
    Ok(runtime_directory.join("saq.sock"))
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Request {
    GetState,
    SetVolume(f32),
    SetMode(u8),
    SetPitchEnabled(bool),
    SetPitch(f32),
    SetSubwoofer(f32),
    SetEqPreset(u8),
    SetEqProfile {
        preset: u8,
        base_preset: u8,
        point_count: u8,
        frequencies_hz: Vec<f32>,
        gains_db: Vec<f32>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Response {
    State {
        volume: f32,
        mode: u8,
        pitch_enabled: bool,
        pitch: f32,
        subwoofer: f32,
        eq_preset: u8,
        eq_base_preset: u8,
        eq_point_count: u8,
        eq_frequencies_hz: Vec<f32>,
        eq_gains_db: Vec<f32>,
    },
    Ok,
    Error(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Event {
    StateUpdated {
        volume: f32,
        mode: u8,
        pitch_enabled: bool,
        pitch: f32,
        subwoofer: f32,
        eq_preset: u8,
        eq_base_preset: u8,
        eq_point_count: u8,
        eq_frequencies_hz: Vec<f32>,
        eq_gains_db: Vec<f32>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    Event(Event),
    Response(Response),
}
