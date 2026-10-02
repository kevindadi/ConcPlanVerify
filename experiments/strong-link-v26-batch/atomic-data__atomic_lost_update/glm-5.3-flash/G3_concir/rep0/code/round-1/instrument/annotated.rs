mod cir_trace;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;

// Shared resource: c (atomic counter), init 0
static C: AtomicI64 = AtomicI64::new(0);

// CIR function: main::rmw_increment
// write_shared { expr: "c + 1", resource: "main::c" }
// Implemented as an atomic read-modify-write retry loop: a failed
// compare-and-swap is retried (never abandoned), so the increment
// always takes effect and appears as a single indivisible step.
fn rmw_increment() {
    let mut current = C.load(Ordering::Acquire);
    loop {
        let next = current + 1;
        match C.compare_exchange_weak(
            current,
            next,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // update succeeded
                return;
            }
            Err(actual) => {
                // failed attempt: retry with the freshly observed value
                current = actual;
            }
        }
    }
}

// CIR function: main::w1
fn w1() {
    rmw_increment();
}

// CIR function: main::w2
fn w2() {
    rmw_increment();
}

fn main() { cir_trace::init();
    // CIR entry main::main:
    // spawn h1 -> main::w1 ; spawn h2 -> main::w2 ; join h1 ; join h2
    let h1 = cir_trace::spawn("h1#1239", w1);
    let h2 = cir_trace::spawn("h2#1271", w2);

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // After joins, read shared state only to print the required terminal line.
    let _final_value = C.load(Ordering::SeqCst);
    let _ = Arc::new(()); // keep imports minimal; no extra synchronization added

    println!("DONE done=1");
 cir_trace::finish();}
