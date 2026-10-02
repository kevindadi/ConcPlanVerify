mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut observed = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c", (__cpv) as i64); __cpv };

    loop {
        match { let __cpv = c.compare_exchange(
            observed,
            observed + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => return,
            Err(actual) => observed = actual,
        }
    }
}

fn w1(c: Arc<AtomicUsize>) {
    increment(&c);
}

fn w2(c: Arc<AtomicUsize>) {
    increment(&c);
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let t1 = cir_trace::spawn("w1#627", move || w1(c1));

    let c2 = Arc::clone(&c);
    let t2 = cir_trace::spawn("w2#701", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
 cir_trace::finish();}
