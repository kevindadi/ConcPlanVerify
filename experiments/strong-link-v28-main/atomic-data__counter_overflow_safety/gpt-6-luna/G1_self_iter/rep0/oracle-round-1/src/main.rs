mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// c is constrained to the declared range 0..=2.
const DECLARED_MAX: u8 = 2;
// Workers use the stricter limit so c can never exceed one.
const EFFECTIVE_MAX: u8 = 1;

struct Counter(u8);

impl Counter {
    fn zero() -> Self {
        Self(0)
    }

    fn value(&self) -> u8 {
        self.0
    }

    fn increment_if_below(&mut self, upper: u8) {
        assert!(upper <= DECLARED_MAX);
        assert!(self.0 <= DECLARED_MAX);

        if self.0 < upper {
            self.0 += 1;
        }
    }
}

struct Shared {
    c: Counter,
}

fn worker(m: Arc<Mutex<Shared>>) {
    let mut shared = m.lock().expect("mutex poisoned");
    shared.c.increment_if_below(EFFECTIVE_MAX);
}

fn w1(m: Arc<Mutex<Shared>>) {
    worker(m);
}

fn w2(m: Arc<Mutex<Shared>>) {
    worker(m);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#867", Shared { c: Counter::zero() }));

    let m1 = Arc::clone(&m);
    let t1 = cir_trace::spawn("w1#947", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = cir_trace::spawn("w2#1021", move || w2(m2));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    let done = m.lock().expect("mutex poisoned").c.value();
    println!("DONE done={done}");
 cir_trace::finish();}
