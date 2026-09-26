mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);
    let done = Arc::new(AtomicUsize::new(0));

    let w1 = {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        cir_trace::spawn("w1", move || {
            {
                let permit = s.acquire();
                done.store(1, Ordering::SeqCst);
                drop(permit);
            }
            {
                let permit = s.acquire();
                done.store(1, Ordering::SeqCst);
                drop(permit);
            }
        })
    };

    let w2 = {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        cir_trace::spawn("w2", move || {
            {
                let permit = s.acquire();
                done.store(1, Ordering::SeqCst);
                drop(permit);
            }
            {
                let permit = s.acquire();
                done.store(1, Ordering::SeqCst);
                drop(permit);
            }
        })
    };

    w1.join().unwrap();
    w2.join().unwrap();
    println!("DONE done={}", done.load(Ordering::SeqCst));
 cir_trace::finish();}
