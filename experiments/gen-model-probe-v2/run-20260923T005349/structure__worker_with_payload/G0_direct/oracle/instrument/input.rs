use std::sync::{Arc, Mutex};
use std::thread;

/// Sequential helper routine: performs only local computation.
///
/// Touches no shared state; it simply produces the value that
/// represents "one unit of work done".
fn compute() -> u64 {
    let mut local: u64 = 0;
    for i in 0..1_000u64 {
        local = local.wrapping_mul(31).wrapping_add(i);
    }
    let _ = local; // local scratch result, discarded
    1
}

/// Worker body used by both w1 and w2.
///
/// Acquires the shared mutex `m` (blocking while another worker holds
/// it), runs the purely local `compute`, then updates the shared
/// variable `acc` while still holding `m`. The mutex is released by
/// dropping the guard before the worker finishes.
fn worker(m: &Mutex<u64>) {
    let mut acc = m.lock().unwrap(); // take m; wait while it is held
    let done = compute();            // sequential, local-only computation
    *acc = done;                     // update shared acc while still holding m
}                                    // guard drops here: m is released

fn main() {
    // `m` is the shared mutex; the u64 it guards is the shared variable `acc`.
    let m = Arc::new(Mutex::new(0u64));

    let m_w1 = Arc::clone(&m);
    let w1 = thread::spawn(move || worker(&m_w1));

    let m_w2 = Arc::clone(&m);
    let w2 = thread::spawn(move || worker(&m_w2));

    w1.join().unwrap();
    w2.join().unwrap();

    let acc = m.lock().unwrap();
    println!("DONE done={}", *acc);
}
