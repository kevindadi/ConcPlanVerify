use std::thread;

fn helper() {
    // No shared state; each call runs independently.
}

fn main() {
    let handle = thread::spawn(helper);
    handle.join().unwrap();

    let handle = thread::spawn(helper);
    handle.join().unwrap();

    println!("DONE done=1");
}
