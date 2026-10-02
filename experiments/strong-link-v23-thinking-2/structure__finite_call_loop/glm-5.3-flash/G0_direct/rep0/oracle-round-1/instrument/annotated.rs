mod cir_trace;
// Roles: helper (auxiliary routine), called by the main task.

// Auxiliary routine: shares no mutexes, counters, or other state
// with the calling task (R2). It runs to completion before returning (R3).
fn helper() {
    // Self-contained work; no shared state, no blocking, always terminates (R4).
    let _ = std::hint::black_box(1 + 1);
}

fn main() { cir_trace::init();
    // First call sequence: main task calls the auxiliary routine (R1).
    helper();

    // Main task then begins the same call sequence again (R1).
    // Because the calls are sequential in one task, each call runs
    // to completion before the next starts (R3), and every schedule
    // terminates (R4).
    helper();

    // Exactly one line of output, then exit (R5).
    println!("DONE done=1");
 cir_trace::finish();}
