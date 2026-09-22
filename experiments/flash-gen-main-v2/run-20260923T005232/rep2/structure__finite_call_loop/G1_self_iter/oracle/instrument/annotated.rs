mod cir_trace;
use std::thread;

fn helper() {
    // Auxiliary routine: no shared state, runs to completion.
}

fn main() { cir_trace::init();
    // R1: main calls helper, then begins the same call sequence again.
    // R3: each call runs to completion before the next starts.
    helper();
    helper();

    // R5: print exactly the required line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
