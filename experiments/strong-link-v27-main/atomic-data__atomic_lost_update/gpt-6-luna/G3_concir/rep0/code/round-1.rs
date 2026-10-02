use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

static c: AtomicI32 = AtomicI32::new(0);

fn increment() {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let observed = match c.compare_exchange(
            expected,
            expected + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(value) | Err(value) => value,
        };

        if observed == expected {
            return;
        }
    }
}

fn w1() {
    increment();
}

fn w2() {
    increment();
}

fn main() {
    let worker1 = thread::spawn(w1);
    let worker2 = thread::spawn(w2);

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
}
