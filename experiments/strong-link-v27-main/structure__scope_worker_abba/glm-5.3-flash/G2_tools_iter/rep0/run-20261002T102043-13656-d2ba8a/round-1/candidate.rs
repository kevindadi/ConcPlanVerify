use std::sync::{Arc, Mutex};
use std::thread;

// Roles: w1, w2 — both workers take the shared resources in the same order.
// Shared resources: a, b — two mutexes contended for by both workers.

fn worker(id: u32, a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // R5: both workers take the mutexes in the same order (a, then b),
    // so no wait cycle can form and every interleaving terminates (R7).
    // R6: lock() blocks until the mutex becomes free if the other worker holds it.

    // Take mutex a.
    let _guard_a = a.lock().unwrap();

    // Take mutex b. At this point this worker holds both a and b (R2),
    // while the other worker can hold neither (R4).

    // Simulate work while holding both mutexes.
    println!("worker {} holds a and b", id);

    // R3: release each mutex once the work is finished (by dropping the guards).
    drop(_guard_b);
    drop(_guard_a);

    println!("worker {} finished", id);
}

fn main() {
    // R1: main task starts a group of two workers contending for mutexes a and b.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    // w1
    let h1 = thread::spawn(move || worker(1, a1, b1));
    // w2
    let h2 = thread::spawn(move || worker(2, a2, b2));

    // R8: the group finishes only after both workers have completed.
    h1.join().unwrap();
    h2.join().unwrap();

    // R9: print exactly the required line, then exit.
    println!("DONE done=1");
}
