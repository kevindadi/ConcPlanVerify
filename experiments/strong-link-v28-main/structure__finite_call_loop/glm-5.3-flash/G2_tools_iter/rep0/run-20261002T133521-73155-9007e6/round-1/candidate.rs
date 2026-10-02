// Auxiliary routine: a pure function with no shared state (R2).
// It performs its work locally and returns, so each call runs to
// completion before the caller proceeds (R3).
fn helper(call_id: u32) {
    // Pure, local computation only -- no mutexes, counters, or
    // shared variables are touched, so no task ever contends
    // for a resource (R2).
    let _result = call_id.wrapping_mul(2);
}

fn main() {
    // R1: the main task performs the same call sequence twice:
    // it calls the auxiliary routine, then begins the identical
    // call again.
    for call_id in 0..2 {
        // Each auxiliary call runs on its own task. The only
        // synchronization is a join handle owned exclusively by
        // the main task, so there is no shared resource and no
        // contention (R2).
        let handle = std::thread::spawn(move || helper(call_id));

        // R3: wait for this auxiliary call to run to completion
        // before the main task starts the next call.
        // Joining always terminates: the helper performs only
        // finite local work, so every schedule/interleaving
        // terminates (R4).
        handle.join().expect("helper task must not panic");
    }

    // R5: print exactly the required line, then exit.
    println!("DONE done=1");
}
