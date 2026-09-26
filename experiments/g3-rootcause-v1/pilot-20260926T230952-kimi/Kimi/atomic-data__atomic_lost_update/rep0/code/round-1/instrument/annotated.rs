mod cir_trace;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: &AtomicI64) {
    let mut cur: i64 = c.load(Ordering::SeqCst);
    loop {
        let obs: i64 = match c.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) => v,
            Err(v) => v,
        };
        if obs == cur {
            return;
        }
        cur = obs;
    }
}

fn w2(c: &AtomicI64) {
    let mut cur: i64 = c.load(Ordering::SeqCst);
    loop {
        let obs: i64 = match c.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) => v,
            Err(v) => v,
        };
        if obs == cur {
            return;
        }
        cur = obs;
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicI64::new(0));

    let c_w1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("w1", move || w1(&c_w1));

    let c_w2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("w2", move || w2(&c_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
