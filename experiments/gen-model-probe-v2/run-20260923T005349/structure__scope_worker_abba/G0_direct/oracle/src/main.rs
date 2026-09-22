mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn worker(a: &Mutex<()>, b: &Mutex<()>) {
    // R5: both workers take the mutexes in the same order (a, then b),
    // so no wait cycle can form and every interleaving terminates (R7).
    // R6: lock() blocks until the mutex becomes free.
    let guard_a = a.lock().unwrap(); // R4: exclusive hold on a
    let guard_b = b.lock().unwrap(); // R4: exclusive hold on b

    // R2: at this point the worker holds both a and b simultaneously.
    // (critical section work)
    let _holds_both = (&guard_a, &guard_b);

    // R3: each mutex is released once the work is finished.
    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    // Shared resources: a (lock), b (lock).
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    // R1: main starts a group of two workers contending for the same a and b.
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(move || worker(&a1, &b1))
        .unwrap();

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(move || worker(&a2, &b2))
        .unwrap();

    // R8: the group finishes only after both workers have completed.
    w1.join().unwrap();
    w2.join().unwrap();

    // R9: print exactly this line and then exit.
    println!("DONE done=1");
 cir_trace::finish();}
