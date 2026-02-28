use std::sync::atomic::{AtomicU32, Ordering};

pub struct AudioState {
    volume: AtomicU32,
    bass_db: AtomicU32,
}

impl AudioState {
    pub fn new(initial_volume: f32) -> Self {
        Self {
            volume: AtomicU32::new(initial_volume.to_bits()),
            bass_db: AtomicU32::new(0.0_f32.to_bits()),
        }
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
    pub fn set_volume(&self, vol: f32) {
        self.volume.store(vol.to_bits(), Ordering::Relaxed);
    }

    pub fn bass_db(&self) -> f32 {
        f32::from_bits(self.bass_db.load(Ordering::Relaxed))
    }
    pub fn set_bass_db(&self, db: f32) {
        self.bass_db.store(db.to_bits(), Ordering::Relaxed);
    }
}
