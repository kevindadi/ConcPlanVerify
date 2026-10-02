mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

struct State {
    permits: usize,
    next_ticket: u64,
    serving: u64,
}

struct Semaphore {
    state: Mutex<State>,
    changed: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(State {
                permits,
                next_ticket: 0,
                serving: 0,
            }),
            changed: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut state = self.state.lock().unwrap();
        let ticket = state.next_ticket;
        state.next_ticket += 1;

        while ticket != state.serving || state.permits == 0 {
            state = self.changed.wait(state).unwrap();
        }

        state.permits -= 1;
        state.serving += 1;
        self.changed.notify_all();

        Permit { semaphore: self }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.permits += 1;
        self.changed.notify_all();
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

fn work() {
    let result = (0..1_000_u64).fold(0, |sum, n| sum.wrapping_add(n));
    std::hint::black_box(result);
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
    let s2 = Arc::clone(&s);

    let handle1 = cir_trace::spawn("w1#1634", move || w1(s1));
    let handle2 = cir_trace::spawn("w2#1688", move || w2(s2));

    let result1 = handle1.join();
    let result2 = handle2.join();

    result1.unwrap();
    result2.unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
