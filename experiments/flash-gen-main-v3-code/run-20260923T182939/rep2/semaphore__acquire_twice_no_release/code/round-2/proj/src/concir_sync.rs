// Generated concir_sync runtime (std only).
#![allow(dead_code)]
use std::sync::{Arc, Condvar, Mutex};

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
        Arc::new(Semaphore { permits: Mutex::new(n), cv: Condvar::new(), name })
    }
    pub fn acquire(&self) -> Permit<'_> {
        let mut p = self.permits.lock().unwrap();
        while *p <= 0 {
            p = self.cv.wait(p).unwrap();
        }
        *p -= 1;
        crate::cir_trace::record("sem_acquire", self.name);
        Permit { sem: self }
    }
    pub fn try_acquire(&self) -> Option<Permit<'_>> {
        let mut p = self.permits.lock().unwrap();
        if *p > 0 {
            *p -= 1;
            crate::cir_trace::record("sem_acquire", self.name);
            Some(Permit { sem: self })
        } else {
            None
        }
    }
    pub fn release(&self) {
        let mut p = self.permits.lock().unwrap();
        *p += 1;
        crate::cir_trace::record("sem_release", self.name);
        self.cv.notify_one();
    }
}

impl<'a> Drop for Permit<'a> {
    fn drop(&mut self) {
        self.sem.release();
    }
}
