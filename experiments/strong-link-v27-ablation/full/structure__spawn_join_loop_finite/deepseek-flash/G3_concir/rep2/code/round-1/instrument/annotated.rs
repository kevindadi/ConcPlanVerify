mod cir_trace;
use std::thread;

fn worker() {
    // R2: no shared work, no shared mutexes or counters.
}

fn main() { cir_trace::init();
    // R1/R3: start worker w1, wait for it exactly once.
    let w1 = cir_trace::spawn("w1#175", worker);
    w1.join().expect("worker w1 panicked");

    // R1/R3: start worker w2, wait for it exactly once.
    let w2 = cir_trace::spawn("w2#313", worker);
    w2.join().expect("worker w2 panicked");

    // R6: required terminal line.
    println!("DONE done=1");
 cir_trace::finish();}
