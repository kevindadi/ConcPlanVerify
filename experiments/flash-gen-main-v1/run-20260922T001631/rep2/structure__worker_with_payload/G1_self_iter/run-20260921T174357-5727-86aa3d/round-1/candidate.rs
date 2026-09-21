use std::sync::{Arc, Mutex};
use std::thread;

fn helper() {
    // sequential helper routine that performs only local computation
    let mut x = 0u64;
    for i in 0..1000 {
        x = x.wrapping_add(i);
    }
    std::hint::black_box(x);
}

fn main() {
    let counter = Arc::new(Mutex::new(0u32));
    let mut handles = Vec::new();

    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            let mut guard = counter.lock().unwrap();
            helper();
            *guard += 1;
            drop(guard);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let done = *counter.lock().unwrap();
    println!("DONE done={}", done);
}
