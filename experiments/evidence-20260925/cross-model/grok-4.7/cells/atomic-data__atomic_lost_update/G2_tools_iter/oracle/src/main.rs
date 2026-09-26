mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));
    // Two permits so both workers can run concurrently; the permit is only a
    // capability, not a lock around the increment.
    let sem = Semaphore::new_named("sem_semaphore0", 2);

    let c1 = Arc::clone(&c);
    let sem1 = Arc::clone(&sem);
    let w1 = cir_trace::spawn("w1", move || {
        let _permit = sem1.acquire();
        let mut v = c1.load(Ordering::SeqCst);
        loop {
            match c1.compare_exchange(v, v + 1, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(actual) => v = actual,
            }
        }
    });

    let c2 = Arc::clone(&c);
    let sem2 = Arc::clone(&sem);
    let w2 = cir_trace::spawn("w2", move || {
        let _permit = sem2.acquire();
        let mut v = c2.load(Ordering::SeqCst);
        loop {
            match c2.compare_exchange(v, v + 1, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(actual) => v = actual,
            }
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
 cir_trace::finish();}
