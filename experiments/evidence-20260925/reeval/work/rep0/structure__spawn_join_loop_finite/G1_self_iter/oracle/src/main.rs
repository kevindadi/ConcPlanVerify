mod cir_trace;
use std::thread;

fn worker() {
    // R2: performs no shared work, shares no mutexes or counters.
}

fn main() { cir_trace::init();
    // R1: start a worker, wait for it, then repeat the same cycle.
    // R3: each worker is joined exactly once before the next is started.
    // R4/R5: joining a worker that always terminates cannot stall.
    for _ in 0..1 {
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
    }

    // R6: print exactly the required line.
    println!("DONE done=1");
 cir_trace::finish();}
