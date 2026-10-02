use std::sync::{Arc, Mutex};
use std::thread;

// s: the shared single-permit semaphore.
// Locking it acquires the permit; unlocking (guard drop) releases it.
// The u32 counts how many activations have completed their work.

fn activation(s: &Mutex<u32>) {
    // R4: if the permit is held by another activation, this call blocks
    // until it becomes available.
    let mut permit = s.lock().unwrap();

    // R2: we now hold the single permit; do the work while holding it.
    *permit += 1;

    // R2/R3: permit is released here when `permit` goes out of scope.
}

fn worker(s: Arc<Mutex<u32>>) {
    // R1: each role may have up to two activations running at once.
    let mut handles = Vec::new();
    for _ in 0..2 {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || activation(&s)));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn main() {
    // Single permit shared by all roles.
    let s = Arc::new(Mutex::new(0u32));

    // R1: main task starts the three worker roles w1, w2, w3.
    let mut roles = Vec::new();
    for _ in 0..3 {
        let s = Arc::clone(&s);
        roles.push(thread::spawn(move || worker(s)));
    }

    // R5/R6: wait for all roles to finish; every activation terminates
    // because the mutex guarantees the permit is eventually released.
    for r in roles {
        r.join().unwrap();
    }

    // Sanity check: all 6 activations (3 roles x 2) completed.
    assert_eq!(*s.lock().unwrap(), 6);

    // R7: print exactly this line, then exit.
    println!("DONE done=1");
}
