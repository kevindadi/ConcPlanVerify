mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: &AtomicI32) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#708", (__cpv) as i64); __cpv };
        let next = old + 1;
        let observed = { let __cpv = c.compare_exchange(old, next, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#708", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv };
        match observed {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2(c: &AtomicI32) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#708", (__cpv) as i64); __cpv };
        let next = old + 1;
        let observed = { let __cpv = c.compare_exchange(old, next, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#708", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv };
        match observed {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));

    let c1 = Arc::clone(&c);
    let h1 = crate::cir_trace::spawn("w1#780", move || w1(&c1));

    let c2 = Arc::clone(&c);
    let h2 = crate::cir_trace::spawn("w2#855", move || w2(&c2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    let done = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#708", (__cpv) as i64); __cpv };
    println!("DONE done={}", done / 2);
 crate::cir_trace::finish();}
