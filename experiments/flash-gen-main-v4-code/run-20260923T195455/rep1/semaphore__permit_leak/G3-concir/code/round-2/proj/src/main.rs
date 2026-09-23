mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let w1 = cir_trace::spawn("w1", move || {
        let _permit = s1.acquire();
        drop(_permit);
    });

    let s2 = Arc::clone(&s);
    let w2 = cir_trace::spawn("w2", move || {
        let _permit = s2.acquire();
        drop(_permit);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
