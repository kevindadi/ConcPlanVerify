use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let observed = match c.compare_exchange(
            expected,
            expected + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(value) => value,
            Err(value) => value,
        };

        if observed == expected {
            return;
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let observed = match c.compare_exchange(
            expected,
            expected + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(value) => value,
            Err(value) => value,
        };

        if observed == expected {
            return;
        }
    }
}

fn main() {
    let c = Arc::new(AtomicI32::new(0));

    let w1_handle = {
        let c = Arc::clone(&c);
        thread::spawn(move || w1(c))
    };
    let w2_handle = {
        let c = Arc::clone(&c);
        thread::spawn(move || w2(c))
    };

    w1_handle.join().expect("w1 panicked");
    w2_handle.join().expect("w2 panicked");

    println!("DONE done=1");
}
