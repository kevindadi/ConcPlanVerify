mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_observed("m#610", State { c: 0 }, __cir_obs_State),
    });

    let worker1 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w1#710", move || w1(shared))
    };

    let worker2 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w2#822", move || w2(shared))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn __cir_obs_State(v: &State, r: &str) { cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
