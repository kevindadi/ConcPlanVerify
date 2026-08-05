//! Reference Rust for tests/e2e/fn_summary_prop/fixed.json.
//! Gold verdict: SAFE with both return goals reachable. A producer writes a
//! mutex-protected value and a consumer reads it; both call helpers that in
//! the CIR are only described by function summaries.

use std::sync::{Arc, Mutex};
use std::thread;

/// Modeled in CIR only via fn_summaries: no reads, no writes.
fn compute() {}

/// Modeled in CIR only via fn_summaries: reads `result`.
fn observe(result: i32) -> i32 {
    result
}

fn main() {
    let result = Arc::new(Mutex::new(0_i32));

    let res_p = Arc::clone(&result);
    let producer = thread::spawn(move || {
        let mut guard = res_p.lock().unwrap(); // s1: lock mtx
        compute(); // s2: call compute
        *guard = 1; // s3: write result = 1
        drop(guard); // s4: drop mtx
    });

    let res_c = Arc::clone(&result);
    let consumer = thread::spawn(move || {
        let guard = res_c.lock().unwrap(); // s1: lock mtx
        let _seen = observe(*guard); // s2: call observe; s3: read result
        drop(guard); // s4: drop mtx
    });

    // Goals: producer.ret and consumer.ret both reachable.
    producer.join().unwrap();
    consumer.join().unwrap();
}
