mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicUsize>) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let observed = match c.compare_exchange(
            expected,
            desired,
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

fn w2(c: Arc<AtomicUsize>) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let observed = match c.compare_exchange(
            expected,
            desired,
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

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));
    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = cir_trace::spawn("w1#1089", move || w1(c1));
    let t2 = cir_trace::spawn("w2#1133", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
