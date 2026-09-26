use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

fn main() {
    let c = AtomicI32::new(0);

    thread::scope(|scope| {
        let h1 = scope.spawn(|| w1(&c));
        let h2 = scope.spawn(|| w2(&c));

        h1.join().unwrap();
        h2.join().unwrap();
    });

    println!("DONE done=1");
}

fn w1(c: &AtomicI32) {
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

fn w2(c: &AtomicI32) {
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
