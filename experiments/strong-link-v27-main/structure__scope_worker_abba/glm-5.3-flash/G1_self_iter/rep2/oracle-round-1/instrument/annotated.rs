mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Entities required by the spec:
//   roles:            w1, w2
//   shared resources: a, b (mutexes)

// R5: single global acquisition order a -> b for every worker.
// No worker ever acquires b before a, so a wait cycle
// (w1 holds a waits b, w2 holds b waits a) cannot form (R7).

fn worker(id: &'static str, a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // R6: lock() blocks until the mutex becomes free.
    // Acquire in fixed order: a first, then b (R5).
    let _ga = a.lock().expect("mutex a poisoned");
    let _gb = b.lock().expect("mutex b poisoned");

    // R2: at this point the worker holds BOTH a and b simultaneously.
    // The guards `_ga` and `_gb` are kept alive by the `_` bindings
    // for the remainder of this scope, so both are genuinely held here.
    println!("[{}] holds a and b", id);

    // R3: `_ga` and `_gb` drop at scope exit, releasing each mutex
    // exactly once, in reverse acquisition order (b then a).
}

fn main() { cir_trace::init();
    // Shared resources; Arc allows both worker threads to access them.
    let a = Arc::new(Mutex::new_named("a_mutex0#1110", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#1148", ()));

    // R1: main starts exactly two workers contending for a and b.
    let h_w1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("worker#1315", move || worker("w1", a, b))
    };
    let h_w2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("worker#1453", move || worker("w2", a, b))
    };

    // R4: mutual exclusion is guaranteed by Mutex itself — at most one
    // worker can hold a given mutex at any moment.

    // R8: the group finishes only after BOTH workers have completed;
    // joining each handle blocks main until that worker terminates.
    h_w1.join().expect("w1 panicked");
    h_w2.join().expect("w2 panicked");

    // R9: exactly one line of output after both workers are done.
    println!("DONE done=1");
 cir_trace::finish();}
