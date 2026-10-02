mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// R1: one main thread plus two worker threads (t1, t2).
// R2: locks `a` and `b` are shared by both workers; a Mutex can be held by only one thread at a time.
// R3: each worker acquires both `a` and `b` before doing its critical work.
// R4: Mutex::lock() blocks while the lock is busy, then proceeds once it becomes free.
// R5: both workers acquire the locks in the same fixed order (a, then b), so a circular
//     wait — and therefore deadlock — is impossible.
// R6: the main thread spawns both workers and joins them before finishing.
// R7: the MutexGuards are dropped (releasing the locks) before each worker's closure ends.
// R8: with a fixed lock order and blocking acquisition, every interleaving terminates.
// R9: the program prints exactly `DONE t1=1 t2=1` and exits.

use std::sync::{Arc};
use std::thread;

fn worker(name: &'static str, a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // Acquire in fixed order: a, then b (prevents deadlock, R5).
    let _guard_a = a.lock().unwrap(); // blocks while busy, then continues (R4)
    let _guard_b = b.lock().unwrap(); // blocks while busy, then continues (R4)

    // Critical work: both locks are held at the same time here (R3).
    println!("{}: holding a and b, doing critical work", name);

    // Guards are dropped here at scope end, releasing both locks (R7).
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#1385", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#1423", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("worker#1512", move || worker("t1", a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle_t2 = cir_trace::spawn("worker#1636", move || worker("t2", a2, b2));

    // Main thread waits for both workers to finish (R6).
    handle_t1.join().unwrap();
    handle_t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
