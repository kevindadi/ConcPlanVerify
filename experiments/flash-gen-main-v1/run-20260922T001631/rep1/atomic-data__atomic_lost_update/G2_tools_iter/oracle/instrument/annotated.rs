mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let counter = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            loop {
                let current = counter.load(Ordering::SeqCst);
                if counter
                    .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done={}", counter.load(Ordering::SeqCst));
 cir_trace::finish();}
