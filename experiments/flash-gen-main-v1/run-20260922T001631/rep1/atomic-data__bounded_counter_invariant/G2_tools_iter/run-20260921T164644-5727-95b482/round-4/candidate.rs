use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let counter = Arc::new(Mutex::new(0u8));

    let c1 = Arc::clone(&counter);
    let c2 = Arc::clone(&counter);

    let h1 = thread::spawn(move || {
        let mut guard = c1.lock().unwrap();
        *guard += 1;
    });

    let h2 = thread::spawn(move || {
        let mut guard = c2.lock().unwrap();
        *guard += 1;
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *counter.lock().unwrap();
    println!("DONE done={}", done);
}
