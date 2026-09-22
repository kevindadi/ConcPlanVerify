mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct ModuleA {
    a: Mutex<()>,
}

struct ModuleB {
    b: Mutex<()>,
}

fn main() { cir_trace::init();
    let module_a = Arc::new(ModuleA { a: Mutex::new_named("module_a_mutex0", ()) });
    let module_b = Arc::new(ModuleB { b: Mutex::new_named("module_b_mutex0", ()) });

    let a_for_t1 = Arc::clone(&module_a);
    let b_for_t1 = Arc::clone(&module_b);

    let a_for_t2 = Arc::clone(&module_a);
    let b_for_t2 = Arc::clone(&module_b);

    let t1 = cir_trace::spawn("t1", move || {
        // t1 runs in module A, depends on resource b owned by module B.
        // Acquire in a global order (a then b) to prevent deadlock.
        let _guard_a = a_for_t1.a.lock().unwrap();
        let _guard_b = b_for_t1.b.lock().unwrap();
        // Hold both resources while performing work.
        // Release happens automatically when guards drop at end of scope.
    });

    let t2 = cir_trace::spawn("t2", move || {
        // t2 runs in module B, depends on resource a owned by module A.
        // Acquire in the same global order (a then b) to prevent deadlock.
        let _guard_a = a_for_t2.a.lock().unwrap();
        let _guard_b = b_for_t2.b.lock().unwrap();
        // Hold both resources while performing work.
        // Release happens automatically when guards drop at end of scope.
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
