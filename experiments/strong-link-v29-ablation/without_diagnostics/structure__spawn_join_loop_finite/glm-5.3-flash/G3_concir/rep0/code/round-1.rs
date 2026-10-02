use std::thread;

fn worker() {
    // The worker performs no shared work; it simply returns.
}

fn main() {
    let w1 = thread::spawn(move || worker());
    w1.join().expect("worker 1 panicked");

    let w2 = thread::spawn(move || worker());
    w2.join().expect("worker 2 panicked");

    println!("DONE done=1");
}
