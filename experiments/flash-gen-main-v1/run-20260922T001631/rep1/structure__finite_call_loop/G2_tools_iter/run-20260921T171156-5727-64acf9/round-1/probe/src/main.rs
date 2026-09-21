fn main() {
    // R1: main task calls an auxiliary routine, then begins the same call sequence again.
    // R2: no shared state between main and auxiliary routine.
    // R3: each auxiliary call runs to completion before the next call starts.
    // R4: every schedule terminates.
    // R5: print exactly "DONE done=1".

    let done = auxiliary();
    let done2 = auxiliary();

    if done && done2 {
        println!("DONE done=1");
    }
}

fn auxiliary() -> bool {
    // No shared state, no synchronization needed; runs to completion.
    true
}
