use std::thread;

fn helper() {
    // Auxiliary routine: no shared state, no output.
}

fn main() {
    // First auxiliary call
    let handle = thread::spawn(helper);
    handle.join().expect("helper thread panicked");

    // Second auxiliary call
    let handle = thread::spawn(helper);
    handle.join().expect("helper thread panicked");

    println!("DONE done=1");
}
