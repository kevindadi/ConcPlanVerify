mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, MutexGuard};
use std::thread;
use std::time::Duration;

struct SemaphoreState {
    permits: usize,
    next_ticket: usize,
    serving_ticket: usize,
}

struct Semaphore {
    state: Mutex<SemaphoreState>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(SemaphoreState {
                permits,
                next_ticket: 0,
                serving_ticket: 0,
            }),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let ticket = state.next_ticket;
        state.next_ticket += 1;

        while state.permits == 0 || ticket != state.serving_ticket {
            state = self
                .available
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }

        state.permits -= 1;
        state.serving_ticket += 1;
        Permit { semaphore: self }
    }
}

struct Permit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        {
            let mut state: MutexGuard<'_, SemaphoreState> = self
                .semaphore
                .state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            state.permits += 1;
        }
        self.semaphore.available.notify_all();
    }
}

fn work() {
    thread::sleep(Duration::from_millis(1));
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        work();
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        work();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let t1 = cir_trace::spawn("w1#1836", move || w1(s1));

    let s2 = Arc::clone(&s);
    let t2 = cir_trace::spawn("w2#1910", move || w2(s2));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
