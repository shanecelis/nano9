use bevy::prelude::*;
use std::{
    fmt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
// Define a newtype around AtomicUsize
pub struct DrawCounter {
    counter: AtomicUsize,
    overflowed: AtomicBool,
}

impl DrawCounter {
    // Create a new DrawCounter with an initial value
    pub const fn new(initial: usize) -> Self {
        Self {
            counter: AtomicUsize::new(initial),
            overflowed: AtomicBool::new(false),
        }
    }

    // Increment the counter and return the previous value
    pub fn increment(&self) -> usize {
        let r = self.counter.fetch_add(1, Ordering::Relaxed);
        if r == 0 {
            warn!("draw counter over flowed.");
            self.overflowed.store(true, Ordering::Relaxed);
        }
        r
    }

    fn overflowed(&self) -> bool {
        self.overflowed.load(Ordering::Relaxed)
    }

    fn reset_overflowed(&self) {
        self.overflowed.store(false, Ordering::Relaxed)
    }

    // Get the current value of the counter
    pub fn get(&self) -> usize {
        self.counter.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: usize) {
        self.counter.store(value, Ordering::Relaxed);
    }
}

impl Default for DrawCounter {
    fn default() -> Self {
        DrawCounter::new(1)
    }
}

impl fmt::Debug for DrawCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DrawCounter({})", self.get())
    }
}
