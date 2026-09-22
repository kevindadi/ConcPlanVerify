use std::thread;

/// Auxiliary routine.
///
/// R2: It touches no mutexes, counters, or any other shared state; it only
/// performs purely local work, so no two tasks ever contend for a resource.
/// R4: The bounded loop always terminates.
fn auxiliary() {
    let mut sum: u64 = 0;
    for i in 0..1024u64 {
        sum = sum.wrapping_add(i);
    }
    std::hint::black_box(sum);
}

fn main() {
    // R1: The main task calls the auxiliary routine...
    let first = thread::spawn(auxiliary);
    // R3: ...and waits for that call to run to completion...
    first.join().expect("first auxiliary call panicked");

    // R1: ...and then begins the same call sequence again.
    let second = thread::spawn(auxiliary);
    // R3: Again, the call completes before the task proceeds.
    second.join().expect("second auxiliary call panicked");

    // R5: Print exactly this line and then exit.
    println!("DONE done=1");
}
