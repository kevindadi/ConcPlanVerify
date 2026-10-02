// R1: one main thread plus two worker threads (t1, t2).
// R2/R3: each worker holds both locks a and b simultaneously during critical work.
// R4: workers block on lock() until the lock is free, then continue.
// R5: both workers acquire the locks in the same order (a, then b),
//     so a circular wait (deadlock) is impossible.
// R6: main starts both workers and joins them before finishing.
// R7: MutexGuards release the locks when dropped at the end of the worker's scope.
// R8: every interleaving terminates because lock acquisition is ordered.
// R9: main prints exactly `DONE t1=1 t2=1` and exits.

use std::sync::{Arc, Mutex};
use std::thread;

fn worker(
    name: &'static str,
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
) {
    // Acquire lock a, then lock b — same order in every worker (R5).
    let _ga = a.lock().unwrap(); // R4: waits until a is free
    let _gb = b.lock().unwrap(); // R4: waits until b is free

    // R3: critical work while holding both locks.
    println!("{}: holding a and b", name);

    // R7: _gb and _ga drop here (end of scope), releasing both locks
    // before the worker finishes.
}

fn main() {
    // Shared resources: a and b (R2: shared by both workers).
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || worker("t1", a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || worker("t2", a2, b2));

    // R6: main waits for both workers to finish.
    t1.join().unwrap();
    t2.join().unwrap();

    // R9: exactly this line, then exit.
    println!("DONE t1=1 t2=1");
}
