mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#838", (__cpv) as i64); __cpv };
        let observed = match { let __cpv = c.compare_exchange(
            old,
            old + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c#838", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(value) | Err(value) => value,
        };
        if observed == old {
            return;
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#838", (__cpv) as i64); __cpv };
        let observed = match { let __cpv = c.compare_exchange(
            old,
            old + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c#838", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(value) | Err(value) => value,
        };
        if observed == old {
            return;
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));
    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = cir_trace::spawn("w1#939", move || w1(c1));
    let t2 = cir_trace::spawn("w2#983", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
