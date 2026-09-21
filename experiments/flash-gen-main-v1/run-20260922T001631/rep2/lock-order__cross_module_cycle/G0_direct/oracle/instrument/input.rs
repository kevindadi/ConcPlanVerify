use std::sync::{Arc, Mutex};
use std::thread;

// Module A owns resource A.
struct ModuleA {
    resource_a: Mutex<()>,
}

// Module B owns resource B.
struct ModuleB {
    resource_b: Mutex<()>,
}

// Task in module A needs both resources.
// It declares a dependency on the resource owned by module B.
fn task_a(module_a: &ModuleA, module_b: &ModuleB) {
    // Acquire in a globally consistent order to prevent deadlock:
    // always acquire resource_a before resource_b.
    let _guard_a = module_a.resource_a.lock().unwrap();
    let _guard_b = module_b.resource_b.lock().unwrap();

    // Hold both resources at the same time while performing work.
    // (Work is represented by the critical section itself.)

    // Both guards are released here, before the task finishes.
}

// Task in module B needs both resources.
// It declares a dependency on the resource owned by module A.
fn task_b(module_b: &ModuleB, module_a: &ModuleA) {
    // Same global lock order: resource_a before resource_b.
    let _guard_a = module_a.resource_a.lock().unwrap();
    let _guard_b = module_b.resource_b.lock().unwrap();

    // Hold both resources at the same time while performing work.

    // Both guards are released here, before the task finishes.
}

fn main() {
    let module_a = Arc::new(ModuleA {
        resource_a: Mutex::new(()),
    });
    let module_b = Arc::new(ModuleB {
        resource_b: Mutex::new(()),
    });

    let a1 = Arc::clone(&module_a);
    let b1 = Arc::clone(&module_b);
    let a2 = Arc::clone(&module_a);
    let b2 = Arc::clone(&module_b);

    // Starting thread launches both tasks and only finishes after both finish.
    let handle_a = thread::spawn(move || {
        task_a(&a1, &b1);
    });

    let handle_b = thread::spawn(move || {
        task_b(&b2, &a2);
    });

    handle_a.join().unwrap();
    handle_b.join().unwrap();

    println!("DONE done=1");
}
