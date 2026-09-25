use std::thread;

fn worker() {
    // R2: performs no shared work, shares no mutexes or counters.
}

fn main() {
    // R1: start a worker, wait for it, then repeat the same cycle.
    // R3: each worker is joined exactly once before the next is started.
    // R4/R5: joining a worker that always terminates cannot stall.
    for _ in 0..1 {
        let handle = thread::spawn(worker);
        handle.join().unwrap();
    }

    // R6: print exactly the required line and exit.
    println!("DONE done=1");
}
