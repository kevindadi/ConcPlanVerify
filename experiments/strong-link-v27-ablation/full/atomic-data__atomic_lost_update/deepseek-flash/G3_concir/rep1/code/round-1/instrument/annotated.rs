mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: &AtomicUsize) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let old = match c.compare_exchange(expected, desired, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) => v,
            Err(v) => v,
        };
        if old != expected {
            continue;
        }
        return;
    }
}

fn w2(c: &AtomicUsize) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let old = match c.compare_exchange(expected, desired, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) => v,
            Err(v) => v,
        };
        if old != expected {
            continue;
        }
        return;
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let h1 = cir_trace::spawn("w1#948", move || {
        w1(&c1);
    });
    let h2 = cir_trace::spawn("w2#1010", move || {
        w2(&c2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
