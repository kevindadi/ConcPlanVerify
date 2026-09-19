mod cir_trace;
// ConcIR worker_payload — standard library only.
//
// Model summary:
//   - resource `m`   : Mutex (sync)
//   - resource `acc` : shared Int variable, initial 0, protected by `m`
//   - main spawns a scope with two closures w1 and w2
//   - each worker: lock m, call compute, acc += 1, unlock m
//
// Trace tags:
//   t0        : main
//   t<s1>_0   : first member of scope s1 (w1)
//   t<s1>_1   : second member of scope s1 (w2)



use std::sync::{Arc, Mutex};
use std::thread;

/// Shared state: the mutex `m` guards the integer variable `acc`.
struct Shared {
    m: Mutex<i64>,
}

impl Shared {
    fn new() -> Self {
        Shared { m: Mutex::new(0) }
    }
}

/// `main::compute` — empty body in the model.
fn compute() {}

/// `main::w1` — closure body.
fn w1(shared: &Arc<Shared>) {
    // s1: mutex_lock(m)
    let mut guard = shared.m.lock().unwrap();
    cir_trace::ev("t<s1>_0", "s1");

    // s2: call compute
    compute();

    // s3: write_shared(acc, acc + 1)
    *guard = *guard + 1;

    // s4: mutex_unlock(m)
    drop(guard);
    cir_trace::ev("t<s1>_0", "s4");

    // s5: return
}

/// `main::w2` — closure body.
fn w2(shared: &Arc<Shared>) {
    // s1: mutex_lock(m)
    let mut guard = shared.m.lock().unwrap();
    cir_trace::ev("t<s1>_1", "s1");

    // s2: call compute
    compute();

    // s3: write_shared(acc, acc + 1)
    *guard = *guard + 1;

    // s4: mutex_unlock(m)
    drop(guard);
    cir_trace::ev("t<s1>_1", "s4");

    // s5: return
}

fn main() {
    let shared = Arc::new(Shared::new());

    // s1: scope { w1, w2 }
    cir_trace::ev("t0", "s1");

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);

    let h1 = thread::spawn(move || w1(&s1));
    let h2 = thread::spawn(move || w2(&s2));

    // join both scope members
    h1.join().unwrap();
    h2.join().unwrap();

    // s2: return
    cir_trace::ev("t0", "s2");

    cir_trace::finish();
}
