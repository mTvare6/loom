use std::sync::atomic::{AtomicU32, Ordering, AtomicBool};

pub struct AudioState {
    volume: AtomicU32,
    bass_db: AtomicU32,
    spatial_mix: AtomicU32,
    bypass: AtomicBool,
}

impl AudioState {
    pub fn new(initial_volume: f32) -> Self {
        Self {
            volume: AtomicU32::new(initial_volume.to_bits()),
            bass_db: AtomicU32::new(0.0_f32.to_bits()),
            spatial_mix: AtomicU32::new(0.0_f32.to_bits()),
            bypass: AtomicBool::new(false),
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

    pub fn spatial_mix(&self) -> f32 {
        f32::from_bits(self.spatial_mix.load(Ordering::Relaxed))
    }
    pub fn set_spatial_mix(&self, mix: f32) {
        self.spatial_mix.store(mix.to_bits(), Ordering::Relaxed);
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass.load(Ordering::Relaxed)
    }
    pub fn set_bypass(&self, state: bool) {
        self.bypass.store(state, Ordering::Relaxed);
    }
}
