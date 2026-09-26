mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    // Shared locks: a and b, each exclusive (one permit).
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);

    // Worker t1: acquires locks in the global order a -> b.
    let a1 = a.clone();
    let b1 = b.clone();
    let t1 = cir_trace::spawn("t1", move || {
        let permit_a = a1.acquire(); // wait until a is free
        let permit_b = b1.acquire(); // wait until b is free
        // Critical work: both locks are held simultaneously.
        let result = 1;
        // Release both locks before finishing.
        drop(permit_b);
        drop(permit_a);
        result
    });

    // Worker t2: uses the same lock order a -> b, so no circular
    // wait (and therefore no deadlock) is possible.
    let a2 = a.clone();
    let b2 = b.clone();
    let t2 = cir_trace::spawn("t2", move || {
        let permit_a = a2.acquire(); // wait until a is free
        let permit_b = b2.acquire(); // wait until b is free
        // Critical work: both locks are held simultaneously.
        let result = 1;
        // Release both locks before finishing.
        drop(permit_b);
        drop(permit_a);
        result
    });

    // Main thread finishes only after both workers have finished.
    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();

    println!("DONE t1={} t2={}", r1, r2);
 cir_trace::finish();}
