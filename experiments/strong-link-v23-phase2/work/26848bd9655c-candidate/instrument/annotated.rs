mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource: s — a counting permit pool (semaphore) starting with exactly one permit.
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            cv: Condvar::new(),
        }
    }

    // Acquire one permit. A waiting worker only blocks itself; the worker
    // currently holding the permit is never blocked by this wait, so it can
    // always release (R5).
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Release one permit and wake a waiting worker so it can proceed (R5).
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

// Each worker acquires and releases the permit a fixed number of times.
// Every acquire is paired with exactly one release on every path (R4),
// and the permit guarantees the two workers never work simultaneously (R3).
// The fixed round count means every interleaving terminates (R6).
const ROUNDS: usize = 3;

fn work_once(worker: &str, round: usize) {
    // Simulated work; no output is produced here so that the program's
    // only printed line is the final DONE line (R7).
    let _ = (worker, round);
}

fn w1(s: &Arc<Semaphore>) {
    for round in 0..ROUNDS {
        s.acquire();
        work_once("w1", round);
        s.release();
    }
}

fn w2(s: &Arc<Semaphore>) {
    for round in 0..ROUNDS {
        s.acquire();
        work_once("w2", round);
        s.release();
    }
}

// Supervising task: launches w1 and w2, waits for both to finish (R1),
// then prints the required final line (R7).
fn supervisor() {
    // One shared permit pool beginning with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let t1 = cir_trace::spawn("w1#2055", move || w1(&s1));

    let s2 = Arc::clone(&s);
    let t2 = cir_trace::spawn("w2#2130", move || w2(&s2));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    println!("DONE done=1");
}

fn main() { cir_trace::init();
    let sup = cir_trace::spawn("sup#2296", supervisor);
    sup.join().expect("supervisor panicked");
 cir_trace::finish();}
