mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource main::c (Var/Int, init 0).
// Encoded as a primitive field named `c` inside a plain struct that is
// stored through its guarding mutex, so every read and write of the
// protected variable is directly observable by the runtime.

struct C {
    c: i32,
}

// CIR function: main::atomic_add_one
//   write_shared { expr: "c + 1", resource: "main::c" }
// Encoded as an atomic read-modify-write retry loop. Each attempt reads
// the current value and writes c + 1 while holding the lock, so the
// update is a single indivisible step and no partial update is ever
// observable. A failed attempt (the observed value no longer matches
// the expected one) is retried, never abandoned, so every worker's
// increment eventually takes effect and the counter always reaches two.
fn atomic_add_one(c: &Arc<Mutex<C>>) {
    let mut expected: i32 = 0;
    loop {
        let mut guard = c.lock().unwrap();
        let current = guard.c;
        if current == expected {
            // Indivisible read-modify-write: c := c + 1
            guard.c = current + 1;
            drop(guard);
            break;
        }
        // Failed update attempt: retry rather than abandon.
        expected = current;
        drop(guard);
        continue;
    }
}

// CIR function: main::w1
fn w1(c: Arc<Mutex<C>>) {
    atomic_add_one(&c);
}

// CIR function: main::w2
fn w2(c: Arc<Mutex<C>>) {
    atomic_add_one(&c);
}

// CIR function: main::main (entry scope)
fn main() { cir_trace::init();
    // Resource main::c, init 0.
    let c: Arc<Mutex<C>> = Arc::new(Mutex::new_observed("c_mutex0#1598", C { c: 0 }, __cir_obs_C));

    // spawn {"func": "main::w1", "handle": "h1"}
    let c1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("w1#1709", move || w1(c1));

    // spawn {"func": "main::w2", "handle": "h2"}
    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("w2#1833", move || w2(c2));

    // join {"handle": "h1"}
    h1.join().expect("w1 panicked");

    // join {"handle": "h2"}
    h2.join().expect("w2 panicked");

    // Terminal line (R9).
    println!("DONE done=1");
 cir_trace::finish();}

fn __cir_obs_C(v: &C, r: &str) { cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
