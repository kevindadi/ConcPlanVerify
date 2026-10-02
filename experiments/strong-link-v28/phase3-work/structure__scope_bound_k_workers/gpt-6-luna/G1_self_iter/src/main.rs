mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct State {
    next_ticket: u64,
    serving: u64,
}

struct Semaphore {
    state: Mutex<State>,
    changed: Condvar,
}

struct Permit<'a> {
    semaphore: &'a Semaphore,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            state: Mutex::new(State {
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

        while state.serving != ticket {
            state = self.changed.wait(state).unwrap();
        }

        drop(state);
        Permit { semaphore: self }
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut state = self.semaphore.state.lock().unwrap();
        state.serving += 1;
        drop(state);
        self.semaphore.changed.notify_all();
    }
}

fn activation(s: &Semaphore) {
    let _permit = s.acquire();

    // Finite work performed while holding the permit.
    for _ in 0..1000 {
        std::hint::spin_loop();
    }
}

fn run_role(s: Arc<Semaphore>) {
    let first = {
        let s = Arc::clone(&s);
        cir_trace::spawn("activation#1283", move || activation(&s))
    };
    let second = {
        let s = Arc::clone(&s);
        cir_trace::spawn("activation#1387", move || activation(&s))
    };

    let first_result = first.join();
    let second_result = second.join();

    first_result.expect("activation panicked");
    second_result.expect("activation panicked");
}

fn w1(s: Arc<Semaphore>) {
    run_role(s);
}

fn w2(s: Arc<Semaphore>) {
    run_role(s);
}

fn w3(s: Arc<Semaphore>) {
    run_role(s);
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new());

    let h1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#1859", move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#1950", move || w2(s))
    };
    let h3 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w3#2041", move || w3(s))
    };

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    h3.join().expect("w3 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
