mod cir_trace;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;

fn cas_add(c: &AtomicIsize) {
    loop {
        let old = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#393", (__cpv) as i64); __cpv };
        if { let __cpv = c
            .compare_exchange_weak(old, old + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#393", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv }
            .is_ok()
        {
            break;
        }
        // failed attempt: retry, never abandon
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicIsize::new(0));

    let c1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("cas_add#467", move || {
        cas_add(&c1);
    });

    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("cas_add#569", move || {
        cas_add(&c2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
