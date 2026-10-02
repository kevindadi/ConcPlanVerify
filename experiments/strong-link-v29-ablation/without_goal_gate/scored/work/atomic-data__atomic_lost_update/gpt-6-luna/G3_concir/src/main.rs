mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#880", (__cpv) as i64); __cpv };
        let seen = match { let __cpv = c.compare_exchange(
            old,
            old + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { crate::cir_trace::record_value("c#880", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(previous) => previous,
            Err(actual) => actual,
        };

        if seen == old {
            return;
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#880", (__cpv) as i64); __cpv };
        let seen = match { let __cpv = c.compare_exchange(
            old,
            old + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { crate::cir_trace::record_value("c#880", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(previous) => previous,
            Err(actual) => actual,
        };

        if seen == old {
            return;
        }
    }
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let handle1 = crate::cir_trace::spawn("w1#987", move || w1(c1));
    let handle2 = crate::cir_trace::spawn("w2#1036", move || w2(c2));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
