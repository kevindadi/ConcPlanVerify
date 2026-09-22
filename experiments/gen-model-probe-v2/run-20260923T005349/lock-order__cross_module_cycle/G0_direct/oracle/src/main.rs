mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Module 1: owns the shared resource `a` and hosts task `t1`.
mod module1 {
    use std::sync::{Arc};
    // R3: task t1 declares its dependency on the resource owned by module2.
    use crate::module2::B;

    /// The shared resource `a` (a lock) owned by this module.
    pub struct A {
        pub lock: Mutex<u64>,
    }

    pub fn new_a() -> Arc<A> {
        Arc::new(A { lock: Mutex::new(0) })
    }

    /// Task t1: needs both `a` (its own module's resource) and `b`
    /// (module2's resource) at the same time.
    pub fn t1(a: Arc<A>, b: Arc<B>) {
        // R5: canonical lock order (a before b) prevents circular wait.
        let mut guard_a = a.lock.lock().unwrap();
        let mut guard_b = b.lock.lock().unwrap();

        // R4: work is performed while holding both resources.
        *guard_a += 1;
        *guard_b += 1;

        // R6: release both resources before finishing.
        drop(guard_b);
        drop(guard_a);
    }
}

/// Module 2: owns the shared resource `b` and hosts task `t2`.
mod module2 {
    use std::sync::{Arc};
    // R3: task t2 declares its dependency on the resource owned by module1.
    use crate::module1::A;

    /// The shared resource `b` (a lock) owned by this module.
    pub struct B {
        pub lock: Mutex<u64>,
    }

    pub fn new_b() -> Arc<B> {
        Arc::new(B { lock: Mutex::new(0) })
    }

    /// Task t2: needs both `b` (its own module's resource) and `a`
    /// (module1's resource) at the same time.
    pub fn t2(a: Arc<A>, b: Arc<B>) {
        // R5: identical canonical lock order (a before b) as t1, so the
        // two tasks can never hold one resource each while waiting on
        // the other — deadlock is impossible in any interleaving.
        let mut guard_a = a.lock.lock().unwrap();
        let mut guard_b = b.lock.lock().unwrap();

        // R4: work is performed while holding both resources.
        *guard_a += 1;
        *guard_b += 1;

        // R6: release both resources before finishing.
        drop(guard_b);
        drop(guard_a);
    }
}

fn main() { cir_trace::init();
    // Each module creates (owns) its shared resource.
    let a = module1::new_a();
    let b = module2::new_b();

    // R7: the starting thread launches both tasks.
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("h1", move || module1::t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("h2", move || module2::t2(a2, b2));

    // R7/R8: wait for both tasks to finish, across both modules.
    h1.join().unwrap();
    h2.join().unwrap();

    // R10: print exactly this line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
