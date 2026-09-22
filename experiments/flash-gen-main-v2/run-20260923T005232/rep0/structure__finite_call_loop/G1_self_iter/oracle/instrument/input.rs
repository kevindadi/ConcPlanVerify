use std::thread;

fn helper() {
    // Auxiliary routine: no shared state, just does its work and returns.
}

fn main() {
    // R1: main calls the auxiliary routine, then begins the same call sequence again.
    // R3: each call runs to completion before the next starts (join ensures this).
    let h1 = thread::spawn(helper);
    h1.join().unwrap();

    let h2 = thread::spawn(helper);
    h2.join().unwrap();

    // R5: print exactly the required line.
    println!("DONE done=1");
}
