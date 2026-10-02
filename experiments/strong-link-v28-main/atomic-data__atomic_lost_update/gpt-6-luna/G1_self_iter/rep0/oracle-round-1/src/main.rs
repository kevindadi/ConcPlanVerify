mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut observed = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#522", (__cpv) as i64); __cpv };

    loop {
        match { let __cpv = c.compare_exchange(
            observed,
            observed + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c#522", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(actual) => observed = actual,
        }
    }
}

fn w1(c: &AtomicUsize) {
    increment(c);
}

fn w2(c: &AtomicUsize) {
    increment(c);
}

fn main() { cir_trace::init();
    let c = AtomicUsize::new(0);

    thread::scope(|s| {
        let t1 = s.spawn(|| w1(&c));
        let t2 = s.spawn(|| w2(&c));

        let r1 = t1.join();
        let r2 = t2.join();
        r1.unwrap();
        r2.unwrap();
    });

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
 cir_trace::finish();}
