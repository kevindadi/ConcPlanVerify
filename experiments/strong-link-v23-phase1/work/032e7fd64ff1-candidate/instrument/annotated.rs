mod cir_trace;
use std::thread;

fn helper() {
    // Auxiliary routine that runs to completion.
    // No shared state, no mutexes, no counters.
}

fn main() { cir_trace::init();
    // R1: A main task calls an auxiliary routine and then begins the same call sequence again.
    // This means: call helper(), then call helper() again.
    
    // First call to helper
    let handle1 = cir_trace::spawn("handle1#353", helper);
    // R3: Each auxiliary call runs to completion before the calling task starts the next call.
    handle1.join().unwrap();
    
    // Second call to helper (same call sequence)
    let handle2 = cir_trace::spawn("handle2#574", helper);
    // R3: Each auxiliary call runs to completion before the calling task starts the next call.
    handle2.join().unwrap();
    
    // R5: The program must print exactly the line `DONE done=1` and then exit.
    println!("DONE done=1");
 cir_trace::finish();}
