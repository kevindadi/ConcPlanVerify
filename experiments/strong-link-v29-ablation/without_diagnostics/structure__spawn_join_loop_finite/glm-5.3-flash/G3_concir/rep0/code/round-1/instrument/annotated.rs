mod cir_trace;
use std::thread;

fn worker() {
    // The worker performs no shared work; it simply returns.
}

fn main() { crate::cir_trace::init();
    let w1 = crate::cir_trace::spawn("worker#122", move || worker());
    w1.join().expect("worker 1 panicked");

    let w2 = crate::cir_trace::spawn("worker#212", move || worker());
    w2.join().expect("worker 2 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
