mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Sequential helper: performs only local computation, no shared state access.
fn compute() -> u64 {
    let mut acc: u64 = 0;
    for i in 0..512u64 {
        acc = acc.wrapping_mul(31).wrapping_add(i);
    }
    acc
}

struct Shared {
    counter: u64,
    sink: u64,
}

fn worker(shared: Arc<Mutex<Shared>>) {
    // Take the shared mutex (blocks until it is free).
    let mut guard = shared.lock().unwrap();
    // Sequential helper doing purely local computation.
    let result = compute();
    // Update the shared counter while still holding the mutex.
    guard.counter += 1;
    guard.sink = guard.sink.wrapping_add(result);
    // Release the mutex before finishing.
    drop(guard);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Mutex::new_named("shared_mutex0", Shared { counter: 0, sink: 0 }));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let s = Arc::clone(&shared);
        handles.push(thread::spawn(move || worker(s)));
    }
    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
