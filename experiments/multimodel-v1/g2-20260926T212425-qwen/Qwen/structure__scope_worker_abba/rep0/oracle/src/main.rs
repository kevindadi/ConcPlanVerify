mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    // We use two semaphores with 1 permit each to act as mutexes A and B.
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);

    let a_clone1 = Arc::clone(&a);
    let b_clone1 = Arc::clone(&b);
    let a_clone2 = Arc::clone(&a);
    let b_clone2 = Arc::clone(&b);

    let handle1 = cir_trace::spawn("handle1", move || {
        // Worker w1: takes A then B (same order as w2)
        let _permit_a = a_clone1.acquire();
        let _permit_b = b_clone1.acquire();
        
        // At this point, both permits are held.
        // Work is done when the function returns / scope ends, releasing permits via Drop.
    });

    let handle2 = cir_trace::spawn("handle2", move || {
        // Worker w2: takes A then B (same order as w1)
        let _permit_a = a_clone2.acquire();
        let _permit_b = b_clone2.acquire();
        
        // At this point, both permits are held.
        // Work is done when the function returns / scope ends, releasing permits via Drop.
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
