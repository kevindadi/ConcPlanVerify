mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let exp = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#646", (__cpv) as i64); __cpv };
        let obs = { let __cpv = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#646", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv };
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let exp = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#646", (__cpv) as i64); __cpv };
        let obs = { let __cpv = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#646", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv };
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));

    let c1 = Arc::clone(&c);
    let h1 = crate::cir_trace::spawn("w1#718", move || w1(c1));
    let c2 = Arc::clone(&c);
    let h2 = crate::cir_trace::spawn("w2#791", move || w2(c2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
