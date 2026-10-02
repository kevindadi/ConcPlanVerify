// R1: The main task calls the auxiliary routine `helper`, then begins the
//     same call sequence again (a second identical call).
// R2: `helper` shares no mutexes, counters, or other state with the main
//     task — there is no shared state at all, so no two tasks contend.
// R3: Each call to `helper` runs to completion before the next call starts,
//     because the calls are plain sequential function calls.
// R4: There is a single task and no concurrency, so every schedule (the
//     only one) terminates.
// R5: The program prints exactly `DONE done=1` and exits.

fn helper() {
    // Auxiliary routine: self-contained, touches no shared state.
}

fn main() {
    // First call sequence.
    helper();

    // Begin the same call sequence again.
    helper();

    // R5: exact required output.
    println!("DONE done=1");
}
