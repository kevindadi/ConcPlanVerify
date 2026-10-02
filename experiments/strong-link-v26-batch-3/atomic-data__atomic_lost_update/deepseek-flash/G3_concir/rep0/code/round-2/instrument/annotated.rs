mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

#[allow(non_upper_case_globals)]
static c: AtomicUsize = AtomicUsize::new(0);

fn increment() {
    loop {
        let old = c.load(Ordering::SeqCst);
        let new = old + 1;
        let seen = match c.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) => v,
            Err(v) => v,
        };
        if seen == old {
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
    let t1 = cir_trace::spawn("w1#545", || w1());
    let t2 = cir_trace::spawn("w2#582", || w2());

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
