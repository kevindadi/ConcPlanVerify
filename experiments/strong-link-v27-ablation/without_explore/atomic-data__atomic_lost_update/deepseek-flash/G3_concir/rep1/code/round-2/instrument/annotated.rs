mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

#[allow(non_upper_case_globals)]
static c: AtomicUsize = AtomicUsize::new(0);

fn w1() {
    let mut cur = c.load(Ordering::SeqCst);
    loop {
        match c.compare_exchange_weak(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return,
            Err(v) => cur = v,
        }
    }
}

fn w2() {
    let mut cur = c.load(Ordering::SeqCst);
    loop {
        match c.compare_exchange_weak(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return,
            Err(v) => cur = v,
        }
    }
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("w1#638", || w1());
    let h2 = cir_trace::spawn("w2#675", || w2());
    h1.join().unwrap();
    h2.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
