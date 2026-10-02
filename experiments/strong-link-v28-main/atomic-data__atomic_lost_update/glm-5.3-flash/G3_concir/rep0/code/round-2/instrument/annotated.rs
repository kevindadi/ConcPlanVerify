mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

// Shared resource: c (atomic counter), starts at zero.
// Stored as an Arc<AtomicI32> named `c` so every load / compare_exchange
// on the simple path receiver `c` is observable by the runtime.

// CIR function: main::atomic_add_one
// write_shared { expr: "c + 1", resource: "main::c" }
// Encoded as an atomic read-modify-write retry loop: a failed
// compare_exchange is retried, never abandoned, so the increment
// always takes effect and is indivisible.
fn atomic_add_one(c: &Arc<AtomicI32>) {
    loop {
        let current = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#1216", (__cpv) as i64); __cpv };
        match { let __cpv = c.compare_exchange(
            current,
            current + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c#1216", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(_) => {
                // Failed update attempt: retry rather than abandon.
                continue;
            }
        }
    }
}

// CIR function: main::w1
fn w1(c: Arc<AtomicI32>) {
    atomic_add_one(&c);
}

// CIR function: main::w2
fn w2(c: Arc<AtomicI32>) {
    atomic_add_one(&c);
}

fn main() { cir_trace::init();
    // Entry scope: resource c, init 0.
    let c: Arc<AtomicI32> = Arc::new(AtomicI32::new(0));

    // spawn {"func": "main::w1", "handle": "h1"}
    let c1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("w1#1338", move || w1(c1));

    // spawn {"func": "main::w2", "handle": "h2"}
    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("w2#1462", move || w2(c2));

    // join {"handle": "h1"}
    h1.join().expect("w1 panicked");

    // join {"handle": "h2"}
    h2.join().expect("w2 panicked");

    // After joins, read shared state only to print the terminal line.
    let _final_value = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#1216", (__cpv) as i64); __cpv };
    println!("DONE done=1");
 cir_trace::finish();}
