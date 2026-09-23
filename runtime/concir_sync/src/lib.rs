//! A standard-library-only counting semaphore for generated Rust.
//!
//! The crate carries no tracing code of its own. `acquire`/`release` invoke a
//! recorder callback if one is installed by the generated `cir_trace` runtime,
//! so the same crate works with or without instrumentation.

use std::sync::{Arc, Condvar, Mutex, OnceLock};

type Recorder = fn(&str, &str);

static RECORDER: OnceLock<Recorder> = OnceLock::new();

/// Install the recorder used for `sem_acquire`/`sem_release` events.
pub fn set_recorder(recorder: Recorder) {
    let _ = RECORDER.set(recorder);
}

fn emit(op: &str, resource: &str) {
    if let Some(recorder) = RECORDER.get() {
        recorder(op, resource);
    }
}

pub struct Semaphore {
    permits: Mutex<i64>,
    cv: Condvar,
    name: &'static str,
}

pub struct Permit<'a> {
    sem: &'a Semaphore,
}

impl Semaphore {
    pub fn new(n: i64) -> Arc<Self> {
        Self::new_named("semaphore", n)
    }

    pub fn new_named(name: &'static str, n: i64) -> Arc<Self> {
        Arc::new(Semaphore {
            permits: Mutex::new(n),
            cv: Condvar::new(),
            name,
        })
    }

    pub fn acquire(&self) -> Permit<'_> {
        let mut p = self.permits.lock().unwrap();
        while *p <= 0 {
            p = self.cv.wait(p).unwrap();
        }
        *p -= 1;
        emit("sem_acquire", self.name);
        Permit { sem: self }
    }

    pub fn try_acquire(&self) -> Option<Permit<'_>> {
        let mut p = self.permits.lock().unwrap();
        if *p > 0 {
            *p -= 1;
            emit("sem_acquire", self.name);
            Some(Permit { sem: self })
        } else {
            None
        }
    }

    fn release_one(&self) {
        let mut p = self.permits.lock().unwrap();
        *p += 1;
        emit("sem_release", self.name);
        self.cv.notify_one();
    }
}

impl<'a> Permit<'a> {
    /// Release explicitly and consume the permit (the drop then does nothing
    /// extra); releasing by dropping the permit is equally valid. There is no
    /// `Semaphore::release`, so a permit cannot be released twice.
    pub fn release(self) {}
}

impl<'a> Drop for Permit<'a> {
    fn drop(&mut self) {
        self.sem.release_one();
    }
}
