mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let expected = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#998", (__cpv) as i64); __cpv };
        let desired = expected + 1;
        let observed = match { let __cpv = c.compare_exchange(
            expected,
            desired,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c#998", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(previous) => previous,
            Err(actual) => actual,
        };

        if observed == expected {
            return;
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let expected = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#998", (__cpv) as i64); __cpv };
        let desired = expected + 1;
        let observed = match { let __cpv = c.compare_exchange(
            expected,
            desired,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c#998", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(previous) => previous,
            Err(actual) => actual,
        };

        if observed == expected {
            return;
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicI32::new(0));

    let c1 = Arc::clone(&c);
    let t1 = cir_trace::spawn("w1#1070", move || w1(c1));

    let c2 = Arc::clone(&c);
    let t2 = cir_trace::spawn("w2#1144", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
