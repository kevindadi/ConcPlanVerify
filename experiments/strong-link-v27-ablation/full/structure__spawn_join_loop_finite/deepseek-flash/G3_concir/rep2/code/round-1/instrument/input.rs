use std::thread;

fn worker() {
    // R2: no shared work, no shared mutexes or counters.
}

fn main() {
    // R1/R3: start worker w1, wait for it exactly once.
    let w1 = thread::spawn(worker);
    w1.join().expect("worker w1 panicked");

    // R1/R3: start worker w2, wait for it exactly once.
    let w2 = thread::spawn(worker);
    w2.join().expect("worker w2 panicked");

    // R6: required terminal line.
    println!("DONE done=1");
}
