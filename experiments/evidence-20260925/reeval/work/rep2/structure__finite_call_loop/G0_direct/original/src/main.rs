use std::thread;

fn helper() {
    // Auxiliary routine: no shared state, no contention.
    // It simply runs to completion.
}

fn main() {
    // R1: main task calls the auxiliary routine, then begins the same call sequence again.
    // R3: each auxiliary call runs to completion before the next call starts.
    // R2: no shared mutexes, counters, or other shared state.
    // R4: every schedule terminates.
    // R5: print exactly "DONE done=1" and exit.

    let h1 = thread::spawn(helper);
    h1.join().unwrap();

    let h2 = thread::spawn(helper);
    h2.join().unwrap();

    println!("DONE done=1");
}
