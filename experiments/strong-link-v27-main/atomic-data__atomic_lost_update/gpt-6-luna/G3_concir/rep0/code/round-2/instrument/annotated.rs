mod cir_trace;
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

fn main() { cir_trace::init();
    let worker1 = cir_trace::spawn("w1#572", || w1());
    let worker2 = cir_trace::spawn("w2#614", || w2());

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
