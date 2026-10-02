mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    let mut old = 0;
    loop {
        let seen = match { let __cpv = c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#758", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(previous) => previous,
            Err(current) => current,
        };
        if seen == old {
            return;
        }
        old = seen;
    }
}

fn w2(c: Arc<AtomicI32>) {
    let mut old = 0;
    loop {
        let seen = match { let __cpv = c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#758", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(previous) => previous,
            Err(current) => current,
        };
        if seen == old {
            return;
        }
        old = seen;
    }
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));
    let w1_c = Arc::clone(&c);
    let w2_c = Arc::clone(&c);

    let t1 = crate::cir_trace::spawn("w1#863", move || w1(w1_c));
    let t2 = crate::cir_trace::spawn("w2#909", move || w2(w2_c));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
