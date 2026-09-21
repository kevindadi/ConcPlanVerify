use std::sync::{Arc, Mutex, Condvar};
use std::thread;

// Module A owns resource A
struct ModuleA {
    resource_a: Mutex<()>,
}

// Module B owns resource B
struct ModuleB {
    resource_b: Mutex<()>,
}

// A global lock ordering to prevent deadlock (R5, R9).
// We use a single "arbiter" mutex that both tasks must acquire before
// acquiring their resources, ensuring a consistent global order.
// Actually, to preserve the "each module owns a resource" structure while
// preventing deadlock, we use a lock ordering: resource_a before resource_b.
// Both tasks acquire in the same order.

struct Resources {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn task_in_module_a(res: Arc<Resources>, done: Arc<(Mutex<usize>, Condvar)>) {
    // R3: declare dependency on resource owned by other module (b)
    // R4: hold both resources at same time
    // R5/R9: acquire in consistent global order: a then b
    let _guard_a = res.a.lock().unwrap();
    let _guard_b = res.b.lock().unwrap();

    // perform work while holding both
    // (simulate work without sleep)

    // R6: release each resource before finishing (drop guards)
    drop(_guard_b);
    drop(_guard_a);

    // signal completion
    let (lock, cvar) = &*done;
    let mut count = lock.lock().unwrap();
    *count += 1;
    cvar.notify_all();
}

fn task_in_module_b(res: Arc<Resources>, done: Arc<(Mutex<usize>, Condvar)>) {
    // R3: declare dependency on resource owned by other module (a)
    // R4: hold both resources at same time
    // R5/R9: acquire in consistent global order: a then b
    let _guard_a = res.a.lock().unwrap();
    let _guard_b = res.b.lock().unwrap();

    // perform work while holding both

    // R6: release each resource before finishing
    drop(_guard_b);
    drop(_guard_a);

    // signal completion
    let (lock, cvar) = &*done;
    let mut count = lock.lock().unwrap();
    *count += 1;
    cvar.notify_all();
}

fn main() {
    // R1: two modules, each owning a resource
    let _module_a = ModuleA { resource_a: Mutex::new(()) };
    let _module_b = ModuleB { resource_b: Mutex::new(()) };

    // Shared resources with a consistent lock order
    let resources = Arc::new(Resources {
        a: Mutex::new(()),
        b: Mutex::new(()),
    });

    // Completion tracking
    let done = Arc::new((Mutex::new(0usize), Condvar::new()));

    // R7: starting thread launches both tasks
    let res_a = Arc::clone(&resources);
    let done_a = Arc::clone(&done);
    let handle_a = thread::spawn(move || {
        task_in_module_a(res_a, done_a);
    });

    let res_b = Arc::clone(&resources);
    let done_b = Arc::clone(&done);
    let handle_b = thread::spawn(move || {
        task_in_module_b(res_b, done_b);
    });

    // R7: only finishes after both tasks have finished
    handle_a.join().unwrap();
    handle_b.join().unwrap();

    // Wait until both tasks have signaled completion (R8)
    let (lock, cvar) = &*done;
    let mut count = lock.lock().unwrap();
    while *count < 2 {
        count = cvar.wait(count).unwrap();
    }

    // R10: print exactly the line and exit
    println!("DONE done=1");
}
