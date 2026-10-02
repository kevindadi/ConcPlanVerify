mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Sequential helper routine: performs only local computation.
/// It takes no shared state, so it can never race or deadlock (R2).
fn compute(seed: u64) -> u64 {
    let mut local = seed;
    for i in 0..10_000u64 {
        local = local.wrapping_mul(6364136223846793005).wrapping_add(i);
    }
    local
}

/// Worker body shared by w1 and w2.
fn worker(m: Arc<Mutex<()>>, acc: Arc<Mutex<u64>>, seed: u64) {
    // R5: `lock()` blocks until the mutex is free; a worker that cannot
    // take a held mutex waits. (Using `try_lock` here would be a defect:
    // it could bail out and violate R2/R5.)
    let guard = m.lock().expect("m poisoned");

    // R2: call the sequential helper while holding m (local work only).
    let _ = compute(seed);

    // R2: update the shared counter while still holding m.
    // acc is itself a Mutex so the later read in main is race-free.
    {
        let mut a = acc.lock().expect("acc poisoned");
        *a = 1;
    } // acc guard released; m still held.

    // R3: release the mutex before the worker finishes.
    drop(guard);
}

fn main() { cir_trace::init();
    // Shared resources: m (lock), acc (shared variable).
    let m = Arc::new(Mutex::new_named("m_mutex0#1224", ()));
    let acc = Arc::new(Mutex::new_named("acc_mutex0#1264", 0u64));

    // R1: main starts a group of two workers, w1 and w2.
    let w1 = {
        let m = Arc::clone(&m);
        let acc = Arc::clone(&acc);
        cir_trace::spawn("worker#1426", move || worker(m, acc, 1))
    };
    let w2 = {
        let m = Arc::clone(&m);
        let acc = Arc::clone(&acc);
        cir_trace::spawn("worker#1565", move || worker(m, acc, 2))
    };

    // R6/R7: joining both workers guarantees that every schedule and
    // interleaving terminates and both workers complete before the
    // result is read. (Printing without joining would be a defect:
    // the output could race with the workers or miss their updates.)
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // R8: after both workers have run, acc == 1 (each worker assigns 1
    // under m; incrementing instead would be a defect, yielding done=2).
    let done = *acc.lock().expect("acc poisoned");
    println!("DONE done={}", done);
 cir_trace::finish();}
