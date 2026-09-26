mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);

    let outer_a = Arc::clone(&a);
    let outer_b = Arc::clone(&b);

    let outer = cir_trace::spawn("clone", move || {
        let x1_a = Arc::clone(&outer_a);
        let x1_b = Arc::clone(&outer_b);
        let x1 = cir_trace::spawn("x1", move || {
            let _a = x1_a.acquire();
            let _b = x1_b.acquire();
        });

        let x2_a = Arc::clone(&outer_a);
        let x2_b = Arc::clone(&outer_b);
        let x2 = cir_trace::spawn("x2", move || {
            let _a = x2_a.acquire();
            let _b = x2_b.acquire();
        });

        x1.join().unwrap();
        x2.join().unwrap();
        1
    });

    let done = outer.join().unwrap();
    println!("DONE done={done}");
 cir_trace::finish();}
