mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: &AtomicI32) {
    let mut observed: i32 = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#1239", (__cpv) as i64); __cpv };
    let mut exp: i32;
    loop {
        // s1: exp = observed
        exp = observed;
        // s2: observed = CAS(c, expected=exp, desired=exp+1)
        observed = match { let __cpv = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#1239", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_prev) => exp,
            Err(actual) => actual,
        };
        // s3: if observed == exp then return else retry
        if observed == exp {
            break;
        }
    }
    // s4: return
}

fn w2(c: &AtomicI32) {
    let mut observed: i32 = { let __cpv = c.load(Ordering::SeqCst); crate::cir_trace::record_value("c#1239", (__cpv) as i64); __cpv };
    let mut exp: i32;
    loop {
        // s1: exp = observed
        exp = observed;
        // s2: observed = CAS(c, expected=exp, desired=exp+1)
        observed = match { let __cpv = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { crate::cir_trace::record_value("c#1239", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_prev) => exp,
            Err(actual) => actual,
        };
        // s3: if observed == exp then return else retry
        if observed == exp {
            break;
        }
    }
    // s4: return
}

fn main() { crate::cir_trace::init();
    // Shared resource: c (atomic counter, init 0)
    let c = Arc::new(AtomicI32::new(0));

    let c_w1 = Arc::clone(&c);
    let handle_w1 = crate::cir_trace::spawn("w1#1320", move || {
        w1(&c_w1);
    });

    let c_w2 = Arc::clone(&c);
    let handle_w2 = crate::cir_trace::spawn("w2#1423", move || {
        w2(&c_w2);
    });

    // Wait for both workers to finish
    handle_w1.join().expect("w1 panicked");
    handle_w2.join().expect("w2 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
