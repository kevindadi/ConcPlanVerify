use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let counter = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            loop {
                let current = counter.load(Ordering::SeqCst);
                let next = current + 1;
                match counter.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(_) => break,
                    Err(_) => continue,
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let done = counter.load(Ordering::SeqCst);
    println!("DONE done={}", done);
}
