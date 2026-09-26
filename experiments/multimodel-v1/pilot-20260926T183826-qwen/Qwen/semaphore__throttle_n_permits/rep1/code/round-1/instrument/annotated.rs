mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);

    let w1 = cir_trace::spawn("w1", move || {
        let _permit = s1.acquire();
        // work performed while holding permit
    });

    let w2 = cir_trace::spawn("w2", move || {
        let _permit = s2.acquire();
        // work performed while holding permit
    });

    let w3 = cir_trace::spawn("w3", move || {
        let _permit = s3.acquire();
        // work performed while holding permit
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
