mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s_w1 = Arc::clone(&s);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        // CIR: main::w1
        let permit = s_w1.acquire();
        drop(permit);
    });

    let s_w2 = Arc::clone(&s);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        // CIR: main::w2
        let permit = s_w2.acquire();
        drop(permit);
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
