mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicUsize>) {
    loop {
        let cur = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#534", (__cpv) as i64); __cpv };
        if { let __cpv = c.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#534", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv }.is_ok() {
            break;
        }
    }
}

fn w2(c: Arc<AtomicUsize>) {
    loop {
        let cur = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#534", (__cpv) as i64); __cpv };
        if { let __cpv = c.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#534", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv }.is_ok() {
            break;
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let h1 = cir_trace::spawn("w1#638", move || w1(c1));
    let h2 = cir_trace::spawn("w2#682", move || w2(c2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#534", (__cpv) as i64); __cpv };
    println!("DONE done={}", done);
 cir_trace::finish();}
