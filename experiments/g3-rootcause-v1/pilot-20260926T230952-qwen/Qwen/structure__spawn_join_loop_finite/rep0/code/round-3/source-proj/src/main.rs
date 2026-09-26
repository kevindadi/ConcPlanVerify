use std::thread;

fn worker() {
    // Worker performs no shared work and returns immediately.
}

fn main() {
    let mut count = 0;
    while count < 1 {
        let h = thread::spawn(worker);
        h.join().unwrap();
        count += 1;
    }
    println!("DONE done=1");
}
