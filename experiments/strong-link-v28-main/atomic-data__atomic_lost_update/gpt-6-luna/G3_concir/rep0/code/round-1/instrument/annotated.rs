mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

static c: AtomicI32 = AtomicI32::new(0);

fn w1() {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let observed = match c.compare_exchange(
            expected,
            desired,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => expected,
            Err(actual) => actual,
        };

        if observed == expected {
            return;
        }
    }
}

fn w2() {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let observed = match c.compare_exchange(
            expected,
            desired,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => expected,
            Err(actual) => actual,
        };

        if observed == expected {
            return;
        }
    }
}

fn main() { cir_trace::init();
    let t1 = cir_trace::spawn("w1#973", move || w1());
    let t2 = cir_trace::spawn("w2#1015", move || w2());

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
