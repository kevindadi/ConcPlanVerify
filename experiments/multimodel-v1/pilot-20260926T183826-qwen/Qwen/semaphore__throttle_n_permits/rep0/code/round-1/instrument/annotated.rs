mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let w1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1", move || {
            let _permit = s.acquire();
        })
    };

    let w2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2", move || {
            let _permit = s.acquire();
        })
    };

    let w3 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w3", move || {
            let _permit = s.acquire();
        })
    };

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
