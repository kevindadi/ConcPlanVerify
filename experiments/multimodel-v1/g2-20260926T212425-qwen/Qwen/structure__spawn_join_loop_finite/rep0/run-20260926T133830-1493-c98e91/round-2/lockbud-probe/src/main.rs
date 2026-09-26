use std::thread;

fn main() {
    // R1: A main task starts a worker task and waits for that worker to finish,
    // then begins the same start-and-wait cycle again.
    // We implement one cycle as specified by R6 (done=1).
    
    // Start the worker task
    let handle = thread::spawn(|| {
        // R2: The worker task performs no shared work and shares no mutexes or counters with the main task.
        // It does nothing except terminate.
    });

    // Wait for the worker to finish
    // R3: Each started worker is waited for exactly once before the next worker is started.
    // R4: No task ever waits for a worker that cannot finish, so no execution stalls.
    // R5: Every schedule and interleaving must terminate.
    handle.join().unwrap();

    // R6: The program must print exactly the line `DONE done=1` and then exit.
    println!("DONE done=1");
}
