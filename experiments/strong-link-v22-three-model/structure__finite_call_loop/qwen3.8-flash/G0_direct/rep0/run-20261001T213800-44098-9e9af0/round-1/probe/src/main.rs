use std::thread;

fn helper() {
    // Auxiliary routine: does some work and returns.
    // No shared state, no contention.
}

fn main() {
    // R1: A main task calls an auxiliary routine and then begins the same call sequence again.
    // This means we call helper(), then call helper() again (same call sequence).
    
    // R3: Each auxiliary call runs to completion before the calling task starts the next call.
    // Since helper is a synchronous function, this is naturally satisfied.
    
    // R2: The main task and the auxiliary routine share no mutexes, counters, or other shared state.
    // Helper takes no arguments and accesses no shared state.
    
    // R4: Every schedule and interleaving terminates.
    // There's only one thread doing sequential calls, so it always terminates.
    
    // R5: Print exactly `DONE done=1` and exit.
    
    // Call helper twice (the "same call sequence" repeated)
    helper();
    helper();
    
    println!("DONE done=1");
}
