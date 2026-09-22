use std::thread;

/// Auxiliary routine.
///
/// R2: It touches no mutexes, counters, or any other shared state, so no two
/// tasks ever contend for a resource.
fn helper() {
    // Purely local work; nothing is shared with the calling task.
    let _local_work = 0u32;
}

fn main() {
    // R1 + R3: The main task calls the auxiliary routine and, only after that
    // call has run to completion (enforced by `join`), begins the same call
    // sequence again.
    let first = thread::spawn(helper);
    first.join().expect("first helper call must not fail");

    let second = thread::spawn(helper);
    second.join().expect("second helper call must not fail");

    // R4: Both helper threads are joined, so every schedule terminates.
    // R5: Print exactly the required line and then exit.
    println!("DONE done=1");
}
