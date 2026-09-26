use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

fn main() {
    let c = Arc::new(AtomicI64::new(0));

    let c1 = Arc::clone(&c);
    let w1 = std::thread::spawn(move || {
        loop {
            let old = c1.load(Ordering::SeqCst);
            let new = old + 1;
            match c1.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    let c2 = Arc::clone(&c);
    let w2 = std::thread::spawn(move || {
        loop {
            let old = c2.load(Ordering::SeqCst);
            let new = old + 1;
            match c2.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
