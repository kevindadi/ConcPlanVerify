mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: two mutexes, a and b, each implemented as a binary
    // semaphore (exactly one permit => at most one worker can hold it).
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);

    // Worker w1: takes a then b (same order as w2, so no wait cycle can
    // form), holds both at the same time, then releases each exactly once.
    let a1 = a.clone();
    let b1 = b.clone();
    let w1 = cir_trace::spawn("w1", move || {
        let permit_a = a1.acquire();
        let permit_b = b1.acquire();
        // Both mutexes a and b are held at the same time here.
        permit_b.release();
        permit_a.release();
    });

    // Worker w2: takes a then b (same order as w1, so no wait cycle can
    // form), holds both at the same time, then releases each exactly once.
    let a2 = a.clone();
    let b2 = b.clone();
    let w2 = cir_trace::spawn("w2", move || {
        let permit_a = a2.acquire();
        let permit_b = b2.acquire();
        // Both mutexes a and b are held at the same time here.
        permit_b.release();
        permit_a.release();
    });

    // The group finishes only after both workers have completed.
    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
