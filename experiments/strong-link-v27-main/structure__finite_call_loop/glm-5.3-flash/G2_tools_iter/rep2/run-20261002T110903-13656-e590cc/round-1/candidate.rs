// Auxiliary routine: a pure function with no shared state.
// It runs to completion on every call and cannot block or fail.
fn helper() {
    // No mutexes, counters, channels, or any other shared resource:
    // everything the routine needs lives entirely on its own stack frame.
    let _work: u64 = 1 + 1;
}

fn main() {
    // R1: main task calls the auxiliary routine, then begins the same
    // call sequence again.
    // R3: each call is a plain sequential function call, so it runs to
    // completion before the next call starts.
    // R2: no state is shared between the calls or with any other task,
    // so no two tasks can ever contend for a resource.
    // R4: there are no loops, no spawns, no locks, and no blocking
    // operations, so every schedule trivially terminates.
    helper();
    helper();

    // R5: print exactly the required line, then exit normally.
    println!("DONE done=1");
}
