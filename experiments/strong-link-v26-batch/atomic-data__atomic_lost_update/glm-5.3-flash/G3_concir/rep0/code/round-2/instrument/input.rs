use std::sync::atomic::{AtomicI64, Ordering};
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

fn main() {
    // CIR entry main::main:
    // spawn h1 -> main::w1 ; spawn h2 -> main::w2 ; join h1 ; join h2
    // Each spawn closure calls the named CIR worker function directly,
    // so the thread entry point is unambiguous.
    let h1 = thread::spawn(|| {
        w1();
    });
    let h2 = thread::spawn(|| {
        w2();
    });

    h1.join().expect("worker thread panicked");
    h2.join().expect("worker thread panicked");

    // After joins, read shared state only to print the required terminal line.
    let _final_value = C.load(Ordering::SeqCst);

    println!("DONE done=1");
}
