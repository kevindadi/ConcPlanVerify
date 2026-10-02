mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

#[derive(Clone, Copy)]
enum Counter {
    Zero,
    One,
    Two,
}

impl Counter {
    fn increment(&mut self) {
        *self = match *self {
            Counter::Zero => Counter::One,
            Counter::One => Counter::Two,
            Counter::Two => unreachable!("the counter cannot exceed two"),
        };
    }
}

struct Shared {
    c: Counter,
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().expect("counter lock poisoned");
    guard.c.increment();
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().expect("counter lock poisoned");
    guard.c.increment();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#688", Shared { c: Counter::Zero }));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let h1 = cir_trace::spawn("w1#796", move || w1(m1));
    let h2 = cir_trace::spawn("w2#840", move || w2(m2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
