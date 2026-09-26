use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

// First module: owns the first shared resource `a`.
mod mod_a {
    use concir_sync::Semaphore;
    use std::sync::Arc;

    // Declaration: task t1 depends on resource `b`, owned by mod_b.
    pub const DEPENDS_ON: &str = "b";

    // The resource owned by this module.
    pub fn resource_a() -> Arc<Semaphore> {
        Semaphore::new(1)
    }

    // Task t1 runs in this module and needs both resources.
    pub fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
        let _depends_on = DEPENDS_ON; // declares dependency on the other module's resource
        // Acquire both resources in the global order a -> b (deadlock-free).
        let permit_a = a.acquire();
        let permit_b = b.acquire();
        // Work is performed while holding both resources at the same time.
        let _work = 1u64 + 1u64;
        // Release each resource before finishing.
        drop(permit_b);
        drop(permit_a);
    }
}

// Second module: owns the second shared resource `b`.
mod mod_b {
    use concir_sync::Semaphore;
    use std::sync::Arc;

    // Declaration: task t2 depends on resource `a`, owned by mod_a.
    pub const DEPENDS_ON: &str = "a";

    // The resource owned by this module.
    pub fn resource_b() -> Arc<Semaphore> {
        Semaphore::new(1)
    }

    // Task t2 runs in this module and needs both resources.
    pub fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
        let _depends_on = DEPENDS_ON; // declares dependency on the other module's resource
        // Acquire both resources in the same global order a -> b (deadlock-free).
        let permit_a = a.acquire();
        let permit_b = b.acquire();
        // Work is performed while holding both resources at the same time.
        let _work = 2u64 + 2u64;
        // Release each resource before finishing.
        drop(permit_b);
        drop(permit_a);
    }
}

fn main() {
    // Each module creates the resource it owns.
    let a = mod_a::resource_a();
    let b = mod_b::resource_b();

    // Launch task t1 (from mod_a).
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || mod_a::t1(a1, b1));

    // Launch task t2 (from mod_b).
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || mod_b::t2(a2, b2));

    // The starting thread finishes only after both tasks have finished.
    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
