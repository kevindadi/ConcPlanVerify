mod cir_trace;
use std::thread;

fn worker() {
    // The worker performs no shared work; it simply returns.
}

fn main() { crate::cir_trace::init();
    // First start-and-wait cycle.
    let w1 = crate::cir_trace::spawn("worker#157", move || worker());
    w1.join().expect("worker w1 panicked");

    // Second start-and-wait cycle.
    let w2 = crate::cir_trace::spawn("worker#284", move || worker());
    w2.join().expect("worker w2 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
