mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut current = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c", (__cpv) as i64); __cpv };

    loop {
        match { let __cpv = c.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(actual) => current = actual,
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

    let worker1 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w1#581", move || w1(c))
    };
    let worker2 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w2#677", move || w2(c))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
