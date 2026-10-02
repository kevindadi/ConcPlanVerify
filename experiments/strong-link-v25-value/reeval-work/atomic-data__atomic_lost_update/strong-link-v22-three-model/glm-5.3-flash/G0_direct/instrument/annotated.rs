mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

static c: AtomicUsize = AtomicUsize::new(0);

fn worker() {
    loop {
        let cur = c.load(Ordering::SeqCst);
        match c.compare_exchange_weak(
            cur,
            cur + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w1() {
    worker();
}

fn w2() {
    worker();
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1#491", w1);
    let h2 = cir_trace::spawn("h2#528", w2);
    h1.join().unwrap();
    h2.join().unwrap();
    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
 cir_trace::finish();}
