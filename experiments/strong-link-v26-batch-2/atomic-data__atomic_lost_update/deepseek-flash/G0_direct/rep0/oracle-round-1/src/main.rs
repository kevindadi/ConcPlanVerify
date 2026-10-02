mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

static c: AtomicUsize = AtomicUsize::new(0);

fn w1() {
    let mut old = c.load(Ordering::SeqCst);
    loop {
        match c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(actual) => old = actual,
        }
    }
}

fn w2() {
    let mut old = c.load(Ordering::SeqCst);
    loop {
        match c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(actual) => old = actual,
        }
    }
}

fn main() { cir_trace::init();
    let t1 = cir_trace::spawn("t1#613", w1);
    let t2 = cir_trace::spawn("t2#645", w2);

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(c.load(Ordering::SeqCst), 2);

    println!("DONE done=1");
 cir_trace::finish();}
