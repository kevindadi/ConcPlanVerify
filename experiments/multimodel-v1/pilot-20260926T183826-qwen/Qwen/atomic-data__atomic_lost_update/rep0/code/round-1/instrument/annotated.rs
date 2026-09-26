mod cir_trace;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

fn main() { cir_trace::init();
    let c = Arc::new(AtomicI64::new(0));

    let c1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("w1", move || {
        loop {
            let old = c1.load(Ordering::SeqCst);
            let new = old + 1;
            match c1.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    let c2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("w2", move || {
        loop {
            let old = c2.load(Ordering::SeqCst);
            let new = old + 1;
            match c2.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
