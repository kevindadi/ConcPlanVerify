mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let mut handles = Vec::new();

    for _ in 0..3 {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            let permit = s.acquire();
            // perform work
            drop(permit);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
