mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let s = s.clone();
            cir_trace::spawn("spawn0", move || {
                let _permit = s.acquire();
                // perform work
                drop(_permit);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
