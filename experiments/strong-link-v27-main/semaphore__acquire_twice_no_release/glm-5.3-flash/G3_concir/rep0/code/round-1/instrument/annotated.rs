mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s: Arc<Semaphore> = Semaphore::new_named("res_semaphore0#117", 1);

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("h1#168", move || {
        let permit = s1.acquire();
        // work happens while holding the permit
        permit.release();
    });

    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("h2#353", move || {
        let permit = s2.acquire();
        // work happens while holding the permit
        permit.release();
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
