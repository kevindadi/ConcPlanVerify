// ConcIR model: worker_payload v3.5.0
// Standard-library-only Rust program with concurrency tracing.

use std::sync::{Arc, Mutex};
use std::thread;

mod cir_trace {
    pub fn ev(_tag: &str, _sid: &str) {
        // Trace hook: no-op in this standard-library-only program.
    }
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let acc = Arc::new(Mutex::new(0i64));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);

    let w1 = move || {
        cir_trace::ev("t1_0", "s1");
        let _guard = m1.lock().unwrap();
        cir_trace::ev("t1_1", "s2");
        compute();
        cir_trace::ev("t1_2", "s3");
        {
            let mut a = acc1.lock().unwrap();
            *a = *a + 1;
        }
        cir_trace::ev("t1_3", "s4");
        drop(_guard);
        cir_trace::ev("t1_4", "s5");
    };

    let w2 = move || {
        cir_trace::ev("t2_0", "s1");
        let _guard = m2.lock().unwrap();
        cir_trace::ev("t2_1", "s2");
        compute();
        cir_trace::ev("t2_2", "s3");
        {
            let mut a = acc2.lock().unwrap();
            *a = *a + 1;
        }
        cir_trace::ev("t2_3", "s4");
        drop(_guard);
        cir_trace::ev("t2_4", "s5");
    };

    cir_trace::ev("t0", "s1");
    let h1 = thread::spawn(w1);
    let h2 = thread::spawn(w2);
    h1.join().unwrap();
    h2.join().unwrap();

    cir_trace::ev("t0", "s2");
}

fn compute() {
    // Empty body per model.
}
