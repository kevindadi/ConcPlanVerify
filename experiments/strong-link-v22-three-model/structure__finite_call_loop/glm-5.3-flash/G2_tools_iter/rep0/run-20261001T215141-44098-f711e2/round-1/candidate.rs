// Entities: role `helper` (auxiliary routine), main task drives the loop.

use std::thread;

/// The auxiliary routine. It takes no arguments and returns nothing;
/// it shares no mutexes, counters, or other state with the main task,
/// so no two tasks ever contend for a resource (R2).
fn helper() {
    // Completes entirely on its own before returning control (R3).
}

/// One "call sequence": invoke the auxiliary routine once.
fn call_sequence() {
    let handle = thread::Builder::new()
        .name("helper-role".to_string())
        .spawn(helper)
        .expect("spawn helper");
    // Joining guarantees the auxiliary call runs to completion
    // before the calling task starts the next call (R3).
    handle.join().expect("helper panicked");
}

fn main() {
    // Main task: call the auxiliary routine, then begin the same
    // call sequence again (R1). A bounded local repetition count
    // ensures every interleaving terminates (R4) while still
    // exercising the repeat.
    let repetitions = 2;
    for _ in 0..repetitions {
        call_sequence();
    }

    // Exactly one line of output, then exit (R5).
    println!("DONE done=1");
}
