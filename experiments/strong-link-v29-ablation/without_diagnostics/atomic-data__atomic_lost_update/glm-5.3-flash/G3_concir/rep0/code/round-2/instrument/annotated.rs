mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: &AtomicI32) {
    loop {
        let exp = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#638", (__cpv) as i64); __cpv };
        let obs = { let __cpv = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#638", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv };
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2(c: &AtomicI32) {
    loop {
        let exp = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#638", (__cpv) as i64); __cpv };
        let obs = { let __cpv = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#638", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv };
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));
    let c_w1 = Arc::clone(&c);
    let c_w2 = Arc::clone(&c);
    let h1 = crate::cir_trace::spawn("w1#742", move || w1(&c_w1));
    let h2 = crate::cir_trace::spawn("w2#789", move || w2(&c_w2));
    h1.join().unwrap();
    h2.join().unwrap();
    let done = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#638", (__cpv) as i64); __cpv };
    println!("DONE done={}", done / 2);
 crate::cir_trace::finish();}
