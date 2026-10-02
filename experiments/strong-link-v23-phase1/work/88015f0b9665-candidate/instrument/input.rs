use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn worker(c: &Arc<AtomicUsize>) {
    // R4/R5: atomic read-modify-write retry loop.
    // R7: compare_exchange is a single indivisible step; no partial update
    // is observable and no update can be silently lost.
    loop {
        let cur = c.load(Ordering::SeqCst);
        match c.compare_exchange(
            cur,
            cur + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,
            Err(_) => continue, // failed attempt retried, never abandoned
        }
    }
}

fn w1(c: &Arc<AtomicUsize>) { worker(c); }
fn w2(c: &Arc<AtomicUsize>) { worker(c); }

fn main() {
    // R2: single shared counter starting at zero
    let c = Arc::new(AtomicUsize::new(0));

    // R1: supervising task launches w1 and w2, then joins both
    let handles = [
        {
            let c = Arc::clone(&c);
            std::thread::spawn(move || w1(&c))
        },
        {
            let c = Arc::clone(&c);
            std::thread::spawn(move || w2(&c))
        },
    ];

    for handle in &handles {
        handle.join().expect("worker panicked");
    }

    // R3/R6: both workers finished, so the counter must be 2 under
    // every schedule (each increment's CAS eventually succeeds).
    debug_assert_eq!(c.load(Ordering::SeqCst), 2);

    // R9: print exactly the required line.
    println!("DONE done=1");
}
