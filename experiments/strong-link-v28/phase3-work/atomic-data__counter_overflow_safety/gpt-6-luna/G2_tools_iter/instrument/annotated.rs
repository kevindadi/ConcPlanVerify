mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

#[repr(u8)]
#[derive(Clone, Copy)]
enum Counter {
    Zero = 0,
    One = 1,
    Two = 2,
}

struct State {
    c: Counter,
}

struct Shared {
    m: Mutex<State>,
}

fn w1(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();
    if (state.c as u8) < 1 {
        state.c = Counter::One;
    }
}

fn w2(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();
    if (state.c as u8) < 1 {
        state.c = Counter::One;
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("m#569", State { c: Counter::Zero }),
    });

    let shared_for_w1 = Arc::clone(&shared);
    let t1 = cir_trace::spawn("w1#669", move || w1(shared_for_w1));

    let shared_for_w2 = Arc::clone(&shared);
    let t2 = cir_trace::spawn("w2#770", move || w2(shared_for_w2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = {
        let state = shared.m.lock().unwrap();
        state.c as u8
    };
    println!("DONE done={done}");
 cir_trace::finish();}
