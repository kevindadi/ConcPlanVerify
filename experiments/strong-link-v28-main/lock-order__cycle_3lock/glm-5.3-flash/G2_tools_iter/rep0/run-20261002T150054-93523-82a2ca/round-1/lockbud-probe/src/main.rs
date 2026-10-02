// Deadlock-free design:
// All workers acquire locks in a single global order: a, then b, then c.
//   t1 needs a and b  -> acquires a, then b
//   t2 needs b and c  -> acquires b, then c
//   t3 needs c and a  -> acquires a, then c   (a first, breaking the cycle)
// Because every worker acquires locks in the same global order, a circular
// wait (each holding one lock and waiting for another) is impossible, so
// every schedule terminates and each worker eventually finishes.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared locks a, b, c.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    // t1: needs a and b; acquires in global order (a, then b).
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let h1 = thread::spawn(move || {
        let _g1 = a1.lock().unwrap();
        let _g2 = b1.lock().unwrap();
        // Critical work: both locks held simultaneously here.
    }); // Both guards drop here: every lock released before t1 finishes.

    // t2: needs b and c; acquires in global order (b, then c).
    let (b2, c2) = (Arc::clone(&b), Arc::clone(&c));
    let h2 = thread::spawn(move || {
        let _g1 = b2.lock().unwrap();
        let _g2 = c2.lock().unwrap();
        // Critical work: both locks held simultaneously here.
    }); // Both guards drop here: every lock released before t2 finishes.

    // t3: needs c and a; acquires in global order (a, then c).
    // Acquiring a before c breaks the a->b->c->a cycle, preventing deadlock.
    let (a3, c3) = (Arc::clone(&a), Arc::clone(&c));
    let h3 = thread::spawn(move || {
        let _g1 = a3.lock().unwrap();
        let _g2 = c3.lock().unwrap();
        // Critical work: both locks held simultaneously here.
    }); // Both guards drop here: every lock released before t3 finishes.

    // Main thread starts all three workers and finishes only after all
    // three have finished (join blocks until each worker terminates).
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
