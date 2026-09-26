use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let w1 = thread::spawn(move || {
        loop {
            let cur = c1.load(Ordering::SeqCst);
            if c1.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                break;
            }
        }
    });

    let c2 = Arc::clone(&c);
    let w2 = thread::spawn(move || {
        loop {
            let cur = c2.load(Ordering::SeqCst);
            if c2.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                break;
            }
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let _ = c.load(Ordering::SeqCst);
    println!("DONE done=1");
}
