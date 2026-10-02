use std::thread;

fn helper() {
    // The auxiliary routine does not share any state with the main task.
}

fn main() {
    // First call of the auxiliary routine.
    let h1 = thread::spawn(|| helper());
    h1.join().unwrap();

    // Begin the same call sequence again: call the auxiliary routine a second time.
    let h2 = thread::spawn(|| helper());
    h2.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
}
