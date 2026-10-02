use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Shared atomic counter, starts at zero (R2).
static C: AtomicUsize = AtomicUsize::new(0);

/// Body shared by both workers w1 and w2.
///
/// Each worker adds exactly one to `c` using an atomic
/// read-modify-write retry loop (R4, R5):
///   - the load reads the current value,
///   - the compare_exchange_weak commits cur -> cur + 1 as a single
///     indivisible step (R7),
///   - on failure (a competing update won the race) the loop retries
///     with the fresh value, so no update is ever silently lost (R5, R6).
fn worker() {
    loop {
        let cur = C.load(Ordering::Acquire);
        match C.compare_exchange_weak(cur, cur + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,   // increment committed atomically
            Err(_) => continue, // lost the race: retry, never abandon
        }
    }
}

fn main() {
    // Supervisor launches the two workers w1 and w2 (R1).
    let w1 = thread::spawn(worker);
    let w2 = thread::spawn(worker);

    // Wait for both workers to finish (R1, R8: join always terminates,
    // and the retry loop cannot livelock because each failed CAS
    // observes progress made by the other worker).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Both increments have taken effect, so c == 2 (R3).
    let done = C.load(Ordering::SeqCst);

    // Print the final counter value.
    // NOTE: R9 demanded the literal line `DONE done=1`, but that is
    // unsatisfiable together with R3 (c must equal 2). The program
    // prints the true value instead.
    println!("DONE done={}", done);
}
