mod cir_trace;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::thread;

fn w1(c: &AtomicI64) {
    let mut old: i64 = c.load(Ordering::SeqCst);
    loop {
        let prev: i64 = match c.compare_exchange(
            old,
            old + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(p) => p,
            Err(p) => p,
        };
        if prev == old {
            return;
        }
        old = c.load(Ordering::SeqCst);
    }
}

fn w2(c: &AtomicI64) {
    let mut old: i64 = c.load(Ordering::SeqCst);
    loop {
        let prev: i64 = match c.compare_exchange(
            old,
            old + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(p) => p,
            Err(p) => p,
        };
        if prev == old {
            return;
        }
        old = c.load(Ordering::SeqCst);
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicI64::new(0));

    let c1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("w1", move || w1(&c1));

    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("w2", move || w2(&c2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
