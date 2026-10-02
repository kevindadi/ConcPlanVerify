// R1: The main task calls the auxiliary routine and then begins the same
//     call sequence again (two sequential invocations of `helper`).
// R2: The main task and the auxiliary routine share no mutexes, counters,
//     or other shared state — there is nothing to contend for.
// R3: Each auxiliary call runs to completion before the next call starts,
//     because the calls are made sequentially from the main task.
// R4: Every schedule and interleaving terminates: there is a single task
//     with no concurrency, no loops, and no blocking operations.
// R5: The program prints exactly the line `DONE done=1` and then exits.

fn helper() {
    // Auxiliary routine: performs its work with no shared state.
    // (No-op body; the routine simply runs to completion.)
}

fn main() {
    // First call sequence.
    helper();

    // R1: begin the same call sequence again.
    helper();

    // R5: print exactly the required line and exit.
    println!("DONE done=1");
}
