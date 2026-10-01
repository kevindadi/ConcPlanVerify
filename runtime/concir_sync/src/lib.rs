//! A standard-library-only counting semaphore for generated Rust.
//!
//! The crate carries no tracing code of its own. `acquire`/`release` invoke a
//! recorder callback if one is installed by the generated `cir_trace` runtime,
//! so the same crate works with or without instrumentation.

use std::sync::{Arc, Condvar, Mutex, OnceLock};

type Recorder = fn(&str, &str);
type CountRecorder = fn(&str, &str, i64);

static RECORDER: OnceLock<Recorder> = OnceLock::new();
static COUNT_RECORDER: OnceLock<CountRecorder> = OnceLock::new();

/// Install the recorder used for `sem_acquire`/`sem_release` events.
pub fn set_recorder(recorder: Recorder) {
    let _ = RECORDER.set(recorder);
}

/// Recorder for explicit counted operations. One call is one event; a count of
/// 2 is not split into two count-1 events.
pub fn set_count_recorder(recorder: CountRecorder) {
    let _ = COUNT_RECORDER.set(recorder);
}

fn emit(op: &str, resource: &str) {
    if let Some(recorder) = RECORDER.get() {
        recorder(op, resource);
    }
}

fn emit_count(op: &str, resource: &str, count: i64) {
    if let Some(recorder) = COUNT_RECORDER.get() {
        recorder(op, resource, count);
    }
}

/// Rejected explicit semaphore count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemCountError {
    NonPositive,
    Overflow,
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

    /// Consume `n` permits. The permits are not returned when this call ends.
    pub fn acquire_count(&self, n: i64) -> Result<(), SemCountError> {
        if n <= 0 {
            return Err(SemCountError::NonPositive);
        }
        let mut permits = self.permits.lock().unwrap();
        while *permits < n {
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= n;
        emit_count("sem_acquire", self.name, n);
        Ok(())
    }

    /// Add `n` permits. This does not undo an earlier `acquire_count`.
    pub fn release_count(&self, n: i64) -> Result<(), SemCountError> {
        if n <= 0 {
            return Err(SemCountError::NonPositive);
        }
        let mut permits = self.permits.lock().unwrap();
        *permits = permits.checked_add(n).ok_or(SemCountError::Overflow)?;
        emit_count("sem_release", self.name, n);
        self.cv.notify_all();
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn two_release_then_acquire_two_from_zero() {
        let sem = Semaphore::new(0);
        sem.release_count(1).unwrap();
        sem.release_count(1).unwrap();
        sem.acquire_count(2).unwrap();
        assert!(sem.try_acquire().is_none());
    }

    #[test]
    fn acquire_two_waits_until_both_permits_exist() {
        let sem = Semaphore::new(0);
        sem.release_count(1).unwrap();
        let started = Arc::new(Mutex::new(false));
        let flag = Arc::clone(&started);
        let worker = {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                *flag.lock().unwrap() = true;
                sem.acquire_count(2).unwrap();
            })
        };
        thread::sleep(Duration::from_millis(50));
        assert!(*started.lock().unwrap());
        assert!(worker.is_finished() == false);
        sem.release_count(1).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn explicit_acquire_is_not_returned_by_drop() {
        let sem = Semaphore::new(2);
        sem.acquire_count(2).unwrap();
        assert!(sem.try_acquire().is_none());
    }

    #[test]
    fn raii_drop_returns_one_permit() {
        let sem = Semaphore::new(1);
        {
            let permit = sem.acquire();
            drop(permit);
        }
        assert!(sem.try_acquire().is_some());
    }

    #[test]
    fn non_positive_count_is_rejected() {
        let sem = Semaphore::new(0);
        assert_eq!(sem.acquire_count(0), Err(SemCountError::NonPositive));
        assert_eq!(sem.release_count(-1), Err(SemCountError::NonPositive));
    }
}
