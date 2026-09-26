use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

// Worker w1: acquires the shared permit twice. Every acquisition is paired
// with exactly one release (the permit's Drop), and the permit is never
// held across a subsequent acquire, so the other worker can always proceed.
fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        // Critical section: work is performed only while holding the permit.
        let mut acc = 0u64;
        for i in 0..10_000u64 {
            acc = acc.wrapping_add(i);
        }
        std::hint::black_box(acc);
        drop(permit); // release exactly once for this acquisition
    }
}

// Worker w2: same acquire/release discipline as w1.
fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        // Critical section: work is performed only while holding the permit.
        let mut acc = 1u64;
        for i in 1..10_000u64 {
            acc = acc.wrapping_mul(i % 7 + 1);
        }
        std::hint::black_box(acc);
        drop(permit); // release exactly once for this acquisition
    }
}

fn main() {
    // One counting permit pool shared by both workers, starting with
    // exactly one permit.
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    // Supervising task launches both workers.
    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));

    // Wait for both workers to finish.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
}
