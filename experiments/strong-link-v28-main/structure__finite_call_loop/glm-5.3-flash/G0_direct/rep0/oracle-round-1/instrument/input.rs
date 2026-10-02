// Roles: helper (auxiliary routine), called by the main task.

// Auxiliary routine: shares no mutexes, counters, or other state with the
// main task (R2). It runs to completion before returning (R3).
fn helper() {
    // No shared state; nothing to contend for.
}

fn main() {
    // R1: the main task calls the auxiliary routine, then begins the same
    // call sequence again.
    helper();
    helper();

    // R4: with a single task and no shared resources, every schedule
    // terminates trivially.
    // R5: print exactly the required line, then exit.
    println!("DONE done=1");
}
