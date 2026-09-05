use loom_dsp::Mode;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

pub struct AudioState {
    volume: AtomicU32,
    mode: AtomicU8,
}

impl AudioState {
    pub fn new(initial_volume: f32) -> Self {
        Self {
            volume: AtomicU32::new(initial_volume.to_bits()),
            mode: AtomicU8::new(Mode::Off as u8),
        }
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
    pub fn set_volume(&self, vol: f32) {
        self.volume.store(vol.to_bits(), Ordering::Relaxed);
    }

    pub fn mode(&self) -> Mode {
        Mode::from_u8(self.mode.load(Ordering::Relaxed))
    }
    pub fn set_mode(&self, mode: Mode) {
        self.mode.store(mode as u8, Ordering::Relaxed);
    }
}

impl loom_pipewire::AudioControls for AudioState {
    fn volume(&self) -> f32 {
        AudioState::volume(self)
    }

    fn mode(&self) -> Mode {
        AudioState::mode(self)
    }
}
