mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);

    let a1 = a.clone();
    let b1 = b.clone();
    let w1 = cir_trace::spawn("w1", move || {
        let _hold_a = a1.acquire();
        let _hold_b = b1.acquire();
    });

    let a2 = a.clone();
    let b2 = b.clone();
    let w2 = cir_trace::spawn("w2", move || {
        let _hold_a = a2.acquire();
        let _hold_b = b2.acquire();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
