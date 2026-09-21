mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Module A owns resource A.
struct ModuleA {
    resource_a: Mutex<()>,
}

// Module B owns resource B.
struct ModuleB {
    resource_b: Mutex<()>,
}

// A global lock ordering prevents deadlock (R5, R9).
// We use a single "ordering" mutex that both tasks must acquire
// before acquiring their resources, ensuring a consistent order.
struct Ordering {
    lock: Mutex<()>,
    cv: Condvar,
}

fn main() { cir_trace::init();
    let module_a = Arc::new(ModuleA {
        resource_a: Mutex::new_named("module_a_mutex0", ()),
    });
    let module_b = Arc::new(ModuleB {
        resource_b: Mutex::new_named("module_b_mutex0", ()),
    });

    // The ordering lock ensures that both tasks acquire resources
    // in a consistent global order, preventing deadlock.
    let ordering = Arc::new(Ordering {
        lock: Mutex::new_named("ordering_mutex0", ()),
        cv: Condvar::new_named("ordering_condvar0"),
    });

    let a_clone = Arc::clone(&module_a);
    let b_clone = Arc::clone(&module_b);
    let ord_clone = Arc::clone(&ordering);

    // Task 1 runs in module A, depends on resource B (R2, R3).
    let task1 = cir_trace::spawn("task1", move || {
        // Acquire the ordering lock first to serialize resource acquisition.
        let _guard = ord_clone.lock.lock().unwrap();

        // Acquire resource A (owned by this module).
        let _ra = a_clone.resource_a.lock().unwrap();
        // Acquire resource B (owned by the other module) — declared dependency.
        let _rb = b_clone.resource_b.lock().unwrap();

        // Hold both resources while performing work (R4).
        // Work is done here.

        // Resources are released when guards drop (R6).
    });

    let a_clone2 = Arc::clone(&module_a);
    let b_clone2 = Arc::clone(&module_b);
    let ord_clone2 = Arc::clone(&ordering);

    // Task 2 runs in module B, depends on resource A (R2, R3).
    let task2 = cir_trace::spawn("task2", move || {
        // Acquire the ordering lock first to serialize resource acquisition.
        let _guard = ord_clone2.lock.lock().unwrap();

        // Acquire resource B (owned by this module).
        let _rb = b_clone2.resource_b.lock().unwrap();
        // Acquire resource A (owned by the other module) — declared dependency.
        let _ra = a_clone2.resource_a.lock().unwrap();

        // Hold both resources while performing work (R4).
        // Work is done here.

        // Resources are released when guards drop (R6).
    });

    // The starting thread launches both tasks and waits for both (R7).
    task1.join().unwrap();
    task2.join().unwrap();

    // R10: print exactly the required line.
    println!("DONE done=1");
 cir_trace::finish();}
