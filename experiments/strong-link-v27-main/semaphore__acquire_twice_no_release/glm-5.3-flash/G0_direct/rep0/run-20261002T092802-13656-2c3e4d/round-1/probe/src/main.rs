use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Shared resource: s — a counting semaphore starting with exactly one permit (R2).
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            available: Condvar::new(),
        }
    }

    // Acquire one permit. A waiting worker only blocks on the condvar tied to
    // the permit count, so the worker currently holding the permit stays fully
    // able to release it (R5).
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Release one permit and wake a waiting worker so it can proceed (R5).
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.available.notify_one();
    }
}

// Worker role: acquires the permit, performs its work while holding it (R3),
// then releases it. The loop means each worker may acquire more than once,
// and the straight-line acquire/release pair guarantees the permit is
// released exactly as many times as it was acquired on every path (R4).
fn worker(s: Arc<Semaphore>, rounds: usize) {
    for _ in 0..rounds {
        s.acquire();
        // Work is performed only while the permit is held (R3).
        s.release();
    }
}

fn w1(s: Arc<Semaphore>) {
    worker(s, 2);
}

fn w2(s: Arc<Semaphore>) {
    worker(s, 2);
}

// Supervising task: launches both worker threads and waits for both to
// finish (R1). Since every acquire is paired with exactly one release and
// the semaphore never deadlocks, every schedule and interleaving of the two
// workers terminates (R6).
fn supervise(s: Arc<Semaphore>) {
    let h1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w2(s))
    };
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    // One shared permit pool with exactly one initial permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let supervisor = thread::spawn(move || supervise(s));
    supervisor.join().unwrap();

    // Exactly one output line, then exit (R7).
    println!("DONE done=1");
}
