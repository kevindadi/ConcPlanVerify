mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);
    let done = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    for _ in 0..3 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || {
            let permit = s.acquire();
            // perform work while holding the permit
            drop(permit);
            done.fetch_add(1, Ordering::SeqCst);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    if done.load(Ordering::SeqCst) == 3 {
        println!("DONE done=1");
    }
 cir_trace::finish();}
