mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicUsize>) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#652", (__cpv) as i64); __cpv };
        let new = old + 1;
        match { let __cpv = c.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#652", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2(c: Arc<AtomicUsize>) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#652", (__cpv) as i64); __cpv };
        let new = old + 1;
        match { let __cpv = c.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#652", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));
    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = cir_trace::spawn("w1#755", move || w1(c1));
    let t2 = cir_trace::spawn("w2#799", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
