mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc};

fn w1(m: &Arc<Mutex<()>>, c: &Arc<AtomicI32>) {
    let _guard = m.lock().unwrap();
    let cur = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#526", (__cpv) as i64); __cpv };
    if cur < 1 {
        { let __cpv = c.store(cur + 1, Ordering::SeqCst); crate::cir_trace::record_value("c#526", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    }
}

fn w2(m: &Arc<Mutex<()>>, c: &Arc<AtomicI32>) {
    let _guard = m.lock().unwrap();
    let cur = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#526", (__cpv) as i64); __cpv };
    if cur < 1 {
        { let __cpv = c.store(cur + 1, Ordering::SeqCst); crate::cir_trace::record_value("c#526", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    }
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#504", ()));
    let c = Arc::new(AtomicI32::new(0));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let h1 = crate::cir_trace::spawn("w1#627", move || w1(&m1, &c1));

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let h2 = crate::cir_trace::spawn("w2#741", move || w2(&m2, &c2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#526", (__cpv) as i64); __cpv };
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
