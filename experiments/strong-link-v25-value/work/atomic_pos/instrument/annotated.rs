mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let observed = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#878", (__cpv) as i64); __cpv };
        let old = { let __cpv = c
            .compare_exchange(
                observed,
                observed + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ); if __cpv.is_ok() { crate::cir_trace::record_value("c#878", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv }
            .unwrap_or_else(|old| old);

        if old == observed {
            return;
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let observed = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#878", (__cpv) as i64); __cpv };
        let old = { let __cpv = c
            .compare_exchange(
                observed,
                observed + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ); if __cpv.is_ok() { crate::cir_trace::record_value("c#878", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv }
            .unwrap_or_else(|old| old);

        if old == observed {
            return;
        }
    }
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));
    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = crate::cir_trace::spawn("w1#979", move || w1(c1));
    let t2 = crate::cir_trace::spawn("w2#1023", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
