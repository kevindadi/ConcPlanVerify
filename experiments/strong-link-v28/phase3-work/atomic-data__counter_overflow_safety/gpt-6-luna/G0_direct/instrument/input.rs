use std::sync::{Arc, Mutex};
use std::thread;

const COUNTER_RANGE_MAX: u8 = 2;

struct State {
    c: u8, // Maintained in the declared range 0..=2.
}

struct Shared {
    m: Mutex<State>,
}

fn w1(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();
    debug_assert!(state.c <= COUNTER_RANGE_MAX);
    if state.c < 1 {
        state.c += 1;
    }
}

fn w2(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();
    debug_assert!(state.c <= COUNTER_RANGE_MAX);
    if state.c < 1 {
        state.c += 1;
    }
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(State { c: 0 }),
    });

    let worker1 = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || w1(shared))
    };

    let worker2 = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || w2(shared))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
}
