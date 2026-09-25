mod cir_trace;
use std::thread;

fn worker() {
    // R2: performs no shared work and shares no mutexes or counters.
}

fn main() { cir_trace::init();
    // R1: start-and-wait cycle performed twice.
    for _ in 0..2 {
        // R3: each started worker is waited for exactly once before the next is started.
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
    }

    // R6: print exactly the required line.
    println!("DONE done=1");
 cir_trace::finish();}
