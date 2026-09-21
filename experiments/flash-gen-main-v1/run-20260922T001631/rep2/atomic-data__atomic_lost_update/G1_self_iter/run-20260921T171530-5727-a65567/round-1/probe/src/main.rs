use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];
    for _ in 0..2 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            let mut cur = c.load(Ordering::SeqCst);
            loop {
                match c.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(_) => break,
                    Err(actual) => cur = actual,
                }
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    println!("DONE done={}", counter.load(Ordering::SeqCst));
}
