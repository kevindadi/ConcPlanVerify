mod cir_trace;
use std::sync::{atomic::{AtomicUsize, Ordering};
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut current = { let __cpv = c.load(Ordering::Relaxed); cir_trace::record_value("c", (__cpv) as i64); __cpv };

    loop {
        match { let __cpv = c.compare_exchange(
            current,
            current + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ); if __cpv.is_ok() { cir_trace::record_value("c", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => return,
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

    let h1 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w1#637", move || w1(c))
    };
    let h2 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w2#728", move || w2(c))
    };

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    assert_eq!(c.load(Ordering::Relaxed), 2);
    println!("DONE done=1");
 cir_trace::finish();}
