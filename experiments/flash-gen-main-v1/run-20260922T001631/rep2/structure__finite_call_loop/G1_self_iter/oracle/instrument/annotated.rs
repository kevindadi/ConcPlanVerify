mod cir_trace;
use std::thread;

fn auxiliary() {
    // R2: no shared state with the main task.
    // R3: this call runs to completion before the caller proceeds.
    println!("auxiliary");
}

fn main() { cir_trace::init();
    // R1: call the auxiliary routine, then begin the same call sequence again.
    auxiliary();
    auxiliary();

    // R5: print exactly the required line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
