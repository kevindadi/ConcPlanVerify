use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Worker role: performs its increment on the shared counter `c`
// using an atomic read-modify-write retry loop (CAS loop), so a
// competing update is never silently lost (R4, R5). Every failed
// attempt is retried (R5), the RMW is indivisible so no partial
// update is ever observable (R7), and the loop always terminates
// because a CAS loop on a single counter cannot livelock under
// any interleaving of w1 and w2 (R6, R8).
fn worker(c: Arc<AtomicUsize>) {
    loop {
        let current = c.load(Ordering::Acquire);
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,   // increment took effect
            Err(_) => continue, // lost the race: retry, never abandon (R5)
        }
    }
}

fn main() {
    // Shared resource `c`: a single atomic counter starting at zero (R2).
    let c = Arc::new(AtomicUsize::new(0));

    // Supervising task: launches the two worker threads w1 and w2
    // and waits for both of them to finish (R1).
    let supervisor = {
        let c = Arc::clone(&c);
        thread::spawn(move || {
            let w1 = {
                let c = Arc::clone(&c);
                thread::spawn(move || worker(c))
            };
            let w2 = {
                let c = Arc::clone(&c);
                thread::spawn(move || worker(c))
            };
            w1.join().expect("w1 panicked");
            w2.join().expect("w2 panicked");
        })
    };

    supervisor.join().expect("supervisor panicked");

    // At this point both workers have finished, so the counter `c`
    // equals two (R3). The program prints exactly the required line
    // and then exits (R9).
    println!("DONE done=1");
}
