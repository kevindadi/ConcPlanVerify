mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, PoisonError};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    fn acquire(self: &Arc<Self>) -> Permit {
        let mut permits = self
            .permits
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        while *permits == 0 {
            permits = self
                .available
                .wait(permits)
                .unwrap_or_else(PoisonError::into_inner);
        }

        *permits -= 1;
        Permit {
            semaphore: Arc::clone(self),
        }
    }
}

struct Permit {
    semaphore: Arc<Semaphore>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut permits = self
            .semaphore
            .permits
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *permits += 1;
        drop(permits);
        self.semaphore.available.notify_one();
    }
}

fn run_worker(s: Arc<Semaphore>) {
    let permit = s.acquire();
    thread::yield_now();
    drop(permit);
}

fn w1(s: Arc<Semaphore>) {
    run_worker(s);
}

fn w2(s: Arc<Semaphore>) {
    run_worker(s);
}

fn w3(s: Arc<Semaphore>) {
    run_worker(s);
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(2));

    let workers = [
        cir_trace::spawn("w1#1454", {
            let s = Arc::clone(&s);
            move || w1(s)
        }),
        cir_trace::spawn("w2#1552", {
            let s = Arc::clone(&s);
            move || w2(s)
        }),
        cir_trace::spawn("w3#1650", {
            let s = Arc::clone(&s);
            move || w3(s)
        }),
    ];

    for worker in workers {
        worker.join().expect("worker thread panicked");
    }

    println!("DONE done=1");
 cir_trace::finish();}
