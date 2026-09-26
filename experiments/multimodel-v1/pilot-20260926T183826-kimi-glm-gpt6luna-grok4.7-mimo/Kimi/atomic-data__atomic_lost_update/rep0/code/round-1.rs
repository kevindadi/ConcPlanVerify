use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::thread;

fn w1(c: &AtomicI64) {
    loop {
        let old = c.load(Ordering::SeqCst);
        let seen = match c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) => v,
            Err(v) => v,
        };
        if seen == old {
            return;
        }
    }
}

fn w2(c: &AtomicI64) {
    loop {
        let old = c.load(Ordering::SeqCst);
        let seen = match c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) => v,
            Err(v) => v,
        };
        if seen == old {
            return;
        }
    }
}

fn main() {
    let c = Arc::new(AtomicI64::new(0));

    let c1 = Arc::clone(&c);
    let h1 = thread::spawn(move || w1(&c1));

    let c2 = Arc::clone(&c);
    let h2 = thread::spawn(move || w2(&c2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
