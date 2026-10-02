use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let observed = match c.compare_exchange(
            expected,
            desired,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(previous) => previous,
            Err(actual) => actual,
        };

        if observed == expected {
            return;
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let observed = match c.compare_exchange(
            expected,
            desired,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(previous) => previous,
            Err(actual) => actual,
        };

        if observed == expected {
            return;
        }
    }
}

fn main() {
    let c = Arc::new(AtomicI32::new(0));

    let c1 = Arc::clone(&c);
    let t1 = thread::spawn(move || w1(c1));

    let c2 = Arc::clone(&c);
    let t2 = thread::spawn(move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
