mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s: Arc<Semaphore> = Semaphore::new_named("res_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let w1 = cir_trace::spawn("w1", move || {
        let permit = s1.acquire();
        // perform work while holding the permit
        permit.release();
    });

    let s2 = Arc::clone(&s);
    let w2 = cir_trace::spawn("w2", move || {
        let permit = s2.acquire();
        // perform work while holding the permit
        permit.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
