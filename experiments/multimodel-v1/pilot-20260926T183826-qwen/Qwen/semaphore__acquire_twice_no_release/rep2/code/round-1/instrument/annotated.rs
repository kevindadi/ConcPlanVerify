mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        let permit = s1.acquire();
        // Work is done while holding the permit.
        drop(permit);
    });

    let s2 = Arc::clone(&s);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        let permit = s2.acquire();
        // Work is done while holding the permit.
        drop(permit);
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
