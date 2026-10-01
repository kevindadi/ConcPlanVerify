mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#101", 1);

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("h1#152", move || {
        let _permit = s1.acquire();
    });

    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("h2#263", move || {
        let _permit = s2.acquire();
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
