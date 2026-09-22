use std::thread;

fn helper() {
    // Auxiliary routine: performs its work and returns.
}

fn main() {
    // R1: main task calls the auxiliary routine, then begins the same call sequence again.
    // R3: each auxiliary call runs to completion before the next call starts.
    let h1 = thread::spawn(helper);
    h1.join().unwrap();

    let h2 = thread::spawn(helper);
    h2.join().unwrap();

    // R5: print exactly the required line and exit.
    println!("DONE done=1");
}
