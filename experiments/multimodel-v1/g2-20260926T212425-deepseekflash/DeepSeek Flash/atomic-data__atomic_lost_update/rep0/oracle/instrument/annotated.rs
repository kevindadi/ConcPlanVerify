mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("w1", move || {
        loop {
            let cur = c1.load(Ordering::SeqCst);
            if c1
                .compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
    });

    let c2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("w2", move || {
        loop {
            let cur = c2.load(Ordering::SeqCst);
            if c2
                .compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done={}", c.load(Ordering::SeqCst));
 cir_trace::finish();}
