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

fn w1(c: Arc<Mutex<Counter>>) {
    let mut m = c.lock().expect("counter lock poisoned");
    m.increment();
}

fn w2(c: Arc<Mutex<Counter>>) {
    let mut m = c.lock().expect("counter lock poisoned");
    m.increment();
}

fn main() { cir_trace::init();
    let c = Arc::new(Mutex::new_named("c_mutex0#635", Counter::Zero));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let h1 = cir_trace::spawn("w1#729", move || w1(c1));
    let h2 = cir_trace::spawn("w2#773", move || w2(c2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
