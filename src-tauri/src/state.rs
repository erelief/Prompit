use std::sync::atomic::{AtomicBool, Ordering};

pub struct WindowConfig {
    pub grow_above: AtomicBool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            grow_above: AtomicBool::new(false),
        }
    }
}

impl WindowConfig {
    pub fn set_grow_above(&self, value: bool) {
        self.grow_above.store(value, Ordering::Relaxed);
    }

    pub fn get_grow_above(&self) -> bool {
        self.grow_above.load(Ordering::Relaxed)
    }
}
