mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));

    let c1 = Arc::clone(&c);
    let h1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(move || w1(&c1))
        .unwrap();

    let c2 = Arc::clone(&c);
    let h2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(move || w2(&c2))
        .unwrap();

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

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
