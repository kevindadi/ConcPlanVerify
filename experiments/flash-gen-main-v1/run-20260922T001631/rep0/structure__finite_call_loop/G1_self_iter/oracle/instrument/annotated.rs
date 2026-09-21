mod cir_trace;
use std::thread;

fn auxiliary() {
    // Auxiliary routine: does some work and returns.
    // No shared state with the main task.
    let _ = 1 + 1;
}

fn main() { cir_trace::init();
    // R1: main calls auxiliary, then begins the same call sequence again.
    // R3: each auxiliary call runs to completion before the next call starts.
    auxiliary();
    auxiliary();

    // R5: print exactly the required line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
