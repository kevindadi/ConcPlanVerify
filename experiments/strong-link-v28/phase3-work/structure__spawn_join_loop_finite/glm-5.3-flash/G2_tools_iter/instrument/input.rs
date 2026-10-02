// No external dependencies required — builds fully offline.
// Cargo.toml needs no [dependencies] entries.

// R2: the worker performs no shared work and shares no mutexes or
// counters with the main task. It touches no shared state at all.
fn worker() {
    // no shared work
}

fn main() {
    // R1: main starts a worker, waits for it to finish, then begins the
    // same start-and-wait cycle again.
    for _ in 0..2 {
        // R3: each worker is started and then joined exactly once,
        // before the next worker is started.
        let handle = std::thread::spawn(worker);

        // R4/R5: the worker body is empty and cannot fail or block, so
        // it always finishes and join always returns — no schedule or
        // interleaving can stall.
        handle.join().expect("worker thread failed");
    }

    // R6: print exactly one line, then exit.
    let done = 1;
    println!("DONE done={}", done);
}
