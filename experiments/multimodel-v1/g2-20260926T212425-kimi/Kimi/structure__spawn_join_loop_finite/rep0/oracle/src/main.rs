mod cir_trace;
use std::thread;

// The worker performs no shared work and shares no mutexes or counters
// with the main task, so it always finishes on its own.
fn worker() {
    // No shared state, no blocking: this task always terminates.
}

fn main() { cir_trace::init();
    // R1/R3: start a worker, wait for it exactly once, then repeat the cycle.
    for _ in 0..2 {
        let handle = cir_trace::spawn("handle", worker);
        handle.join().expect("worker must finish without panicking");
    }

    // R6: print exactly this line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
