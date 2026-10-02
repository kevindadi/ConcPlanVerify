use std::thread;

fn worker() {
    // The worker performs no shared work; it simply returns.
}

fn main() {
    // First start-and-wait cycle.
    let w1 = thread::spawn(move || worker());
    w1.join().expect("worker w1 panicked");

    // Second start-and-wait cycle.
    let w2 = thread::spawn(move || worker());
    w2.join().expect("worker w2 panicked");

    println!("DONE done=1");
}
