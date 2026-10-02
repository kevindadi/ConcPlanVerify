mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc};
use std::thread;

// Shared resources of the design:
//   m : Mutex (sync resource)
//   c : shared counter (var protected by m), starts at 0
// The counter is represented atomically so that each increment is a single
// indivisible read-modify-write step; the mutex m is taken around the update
// exactly as the design's critical section prescribes.

fn w1(m: Arc<Mutex<()>>, c: Arc<AtomicIsize>) {
    // mutex_lock main::m
    let _guard = m.lock().unwrap();
    // retry loop: read c, write c = old + 1 as one atomic RMW step
    let mut old;
    loop {
        old = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c", (__cpv) as i64); __cpv };
        match { let __cpv = c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(_) => continue, // failed attempt is retried, never abandoned
        }
    }
    // mutex_unlock main::m (guard drops at end of scope)
    drop(_guard);
}

fn w2(m: Arc<Mutex<()>>, c: Arc<AtomicIsize>) {
    // mutex_lock main::m
    let _guard = m.lock().unwrap();
    // retry loop: read c, write c = old + 1 as one atomic RMW step
    let mut old;
    loop {
        old = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c", (__cpv) as i64); __cpv };
        match { let __cpv = c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(_) => continue, // failed attempt is retried, never abandoned
        }
    }
    // mutex_unlock main::m (guard drops at end of scope)
    drop(_guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1534", ()));
    let c = Arc::new(AtomicIsize::new(0));

    // spawn main::w1
    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("w1#1681", move || w1(m1, c1));

    // spawn main::w2
    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("w2#1810", move || w2(m2, c2));

    // join; join
    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
