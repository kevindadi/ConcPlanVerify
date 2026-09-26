mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let w1 = cir_trace::spawn("w1", move || {
        // Acquire permit, do work, release permit.
        // The permit is released when the guard goes out of scope.
        let _permit = s1.acquire();
        // Work happens while holding the permit
        drop(_permit);
    });

    let s2 = Arc::clone(&s);
    let w2 = cir_trace::spawn("w2", move || {
        // Acquire permit, do work, release permit.
        let _permit = s2.acquire();
        // Work happens while holding the permit
        drop(_permit);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
