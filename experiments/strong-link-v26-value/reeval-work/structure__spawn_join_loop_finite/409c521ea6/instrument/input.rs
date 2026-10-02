use std::thread;

fn worker() {
    // R2: The worker performs no shared work and shares no state with main.
}

fn main() {
    // R1: Start a worker and wait for it to finish, then repeat the cycle.
    // R3: Each worker is joined exactly once before the next is started.
    // R4/R5: Every worker can finish, so no stall; all schedules terminate.
    for _ in 0..2 {
        let handle = thread::spawn(worker);
        handle.join().expect("worker panicked");
    }

    // R6: Print exactly the required line and exit.
    println!("DONE done=1");
}
