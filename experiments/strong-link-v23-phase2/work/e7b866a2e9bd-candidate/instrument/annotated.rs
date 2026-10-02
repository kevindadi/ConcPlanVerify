mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct State {
    permits: usize,
    next_ticket: u64,
    serving_ticket: u64,
}

struct Semaphore {
    state: Mutex<State>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(State {
                permits,
                next_ticket: 0,
                serving_ticket: 0,
            }),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut state = self.state.lock().unwrap();
        let ticket = state.next_ticket;
        state.next_ticket += 1;

        while ticket != state.serving_ticket || state.permits == 0 {
            state = self.available.wait(state).unwrap();
        }

        state.permits -= 1;
        Permit { semaphore: self }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.permits += 1;
        state.serving_ticket += 1;
        drop(state);

        self.available.notify_all();
    }
}

struct Permit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        thread::yield_now();
        drop(permit);
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        thread::yield_now();
        drop(permit);
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let t1 = cir_trace::spawn("w1#1586", move || w1(s1));

    let s2 = Arc::clone(&s);
    let t2 = cir_trace::spawn("w2#1660", move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
