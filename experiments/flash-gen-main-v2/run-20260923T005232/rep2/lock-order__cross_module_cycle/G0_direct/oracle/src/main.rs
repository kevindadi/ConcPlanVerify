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

    let a1 = Arc::clone(&module_a);
    let b1 = Arc::clone(&module_b);

    let a2 = Arc::clone(&module_a);
    let b2 = Arc::clone(&module_b);

    // t1 runs in module A, depends on resource b owned by module B.
    let t1 = cir_trace::spawn("t1", move || {
        // Acquire in a globally consistent order: a then b.
        let _ga = a1.a.lock().unwrap();
        let _gb = b1.b.lock().unwrap();
        // Hold both resources while performing work.
        // Release happens automatically at end of scope.
    });

    // t2 runs in module B, depends on resource a owned by module A.
    let t2 = cir_trace::spawn("t2", move || {
        // Same global order: a then b, preventing deadlock.
        let _ga = a2.a.lock().unwrap();
        let _gb = b2.b.lock().unwrap();
        // Hold both resources while performing work.
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
