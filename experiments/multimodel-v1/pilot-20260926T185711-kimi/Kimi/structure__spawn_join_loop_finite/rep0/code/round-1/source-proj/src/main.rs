use std::thread;

fn worker() {
    // no operations
}

fn main() {
    let h1 = thread::spawn(worker);
    h1.join().unwrap();
    let h2 = thread::spawn(worker);
    h2.join().unwrap();
    let done = 1;
    println!("DONE done={}", done);
}
