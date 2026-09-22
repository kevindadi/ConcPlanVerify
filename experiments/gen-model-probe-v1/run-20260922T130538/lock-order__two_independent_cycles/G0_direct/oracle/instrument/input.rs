use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Four locks arranged as two independent pairs.
    // First pair: (p1a, p1b). Second pair: (p2a, p2b).
    let p1a = Arc::new(Mutex::new(0u64));
    let p1b = Arc::new(Mutex::new(0u64));
    let p2a = Arc::new(Mutex::new(0u64));
    let p2b = Arc::new(Mutex::new(0u64));

    let mut handles = Vec::with_capacity(4);

    // Workers 1 and 2: both need the first pair.
    // Both acquire in the SAME relative order: p1a first, then p1b.
    // This consistent ordering makes deadlock impossible (R6, R8, R11).
    for _ in 0..2 {
        let first = Arc::clone(&p1a);
        let second = Arc::clone(&p1b);
        handles.push(thread::spawn(move || {
            let mut g1 = first.lock().unwrap();
            let mut g2 = second.lock().unwrap();
            // Work while holding BOTH locks of the pair simultaneously (R4).
            *g1 += 1;
            *g2 += 1;
            // Guards drop here, releasing both locks before finishing (R9).
        }));
    }

    // Workers 3 and 4: both need the second pair, which is fully
    // independent of the first pair (R5).
    // Both acquire in the SAME relative order: p2a first, then p2b (R7).
    for _ in 0..2 {
        let first = Arc::clone(&p2a);
        let second = Arc::clone(&p2b);
        handles.push(thread::spawn(move || {
            let mut g1 = first.lock().unwrap();
            let mut g2 = second.lock().unwrap();
            // Work while holding BOTH locks of the pair simultaneously (R4).
            *g1 += 1;
            *g2 += 1;
            // Guards drop here, releasing both locks before finishing (R9).
        }));
    }

    // Main finishes only after all four workers have finished (R10).
    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
