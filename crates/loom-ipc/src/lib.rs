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
    let directories = ProjectDirs::from("com", "epestr", "loom")
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "home directory is unavailable"))?;
    let runtime_directory = directories
        .runtime_dir()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "User runtime directory found"))?;
    Ok(runtime_directory.join("loom.sock"))
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Request {
    GetState,
    SetVolume(f32),
    SetMode(u8),
    SetPitchEnabled(bool),
    SetPitch(f32),
    SetSubwoofer(f32),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Response {
    State {
        volume: f32,
        mode: u8,
        pitch_enabled: bool,
        pitch: f32,
        subwoofer: f32,
    },
    Ok,
    Error,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Event {
    StateUpdated {
        volume: f32,
        mode: u8,
        pitch_enabled: bool,
        pitch: f32,
        subwoofer: f32,
    },
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Frame {
    Event(Event),
    Response(Response),
}
