mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let w1 = cir_trace::spawn("w1", move || {
        let permit = s1.acquire();
        // Work is done while holding the permit.
        drop(permit);
    });

    let s2 = Arc::clone(&s);
    let w2 = cir_trace::spawn("w2", move || {
        let permit = s2.acquire();
        // Work is done while holding the permit.
        drop(permit);
    });

    let s3 = Arc::clone(&s);
    let w3 = cir_trace::spawn("w3", move || {
        let permit = s3.acquire();
        // Work is done while holding the permit.
        drop(permit);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
