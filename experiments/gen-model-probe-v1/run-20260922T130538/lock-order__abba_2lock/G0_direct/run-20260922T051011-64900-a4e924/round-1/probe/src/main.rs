use std::sync::{Arc, Mutex};
use std::thread;

// Each worker acquires the two shared locks in the SAME fixed order
// (lock_a before lock_b). This global ordering breaks the circular-wait
// condition, so deadlock (R5) is impossible and every interleaving
// terminates (R8). A worker that finds a lock busy simply blocks in
// `lock()` until it becomes free (R4).
fn worker(lock_a: Arc<Mutex<()>>, lock_b: Arc<Mutex<()>>) -> i32 {
    // Acquire both locks; both are held simultaneously (R3).
    let _guard_a = lock_a.lock().unwrap();
    let _guard_b = lock_b.lock().unwrap();

    // Critical work performed while holding both locks.
    let result = 1;

    // Guards are dropped here, releasing both locks before the
    // worker finishes (R7).
    result
}

fn main() {
    // The two locks shared by both workers (R2). A Mutex can be held
    // by at most one worker at a time.
    let lock_a = Arc::new(Mutex::new(()));
    let lock_b = Arc::new(Mutex::new(()));

    // Main thread starts both workers (R1, R6).
    let (a1, b1) = (Arc::clone(&lock_a), Arc::clone(&lock_b));
    let t1 = thread::spawn(move || worker(a1, b1));

    let (a2, b2) = (Arc::clone(&lock_a), Arc::clone(&lock_b));
    let t2 = thread::spawn(move || worker(a2, b2));

    // Main finishes only after both workers have finished (R6).
    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();

    // Exactly one line of output (R9).
    println!("DONE t1={} t2={}", r1, r2);
}
