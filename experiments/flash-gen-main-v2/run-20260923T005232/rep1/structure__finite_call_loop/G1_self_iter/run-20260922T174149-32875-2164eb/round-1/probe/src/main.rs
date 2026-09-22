use std::thread;

fn helper() {
    // Auxiliary routine: no shared state, runs to completion.
}

fn main() {
    // R1: main calls the auxiliary routine, then begins the same call sequence again.
    // R3: each auxiliary call runs to completion before the next call starts.
    let h1 = thread::spawn(helper);
    h1.join().unwrap();

    let h2 = thread::spawn(helper);
    h2.join().unwrap();

    // R5: print exactly the required line.
    println!("DONE done=1");
}
