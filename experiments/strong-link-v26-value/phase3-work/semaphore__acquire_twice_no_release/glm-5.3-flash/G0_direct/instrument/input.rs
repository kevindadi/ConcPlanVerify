use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Shared resource: s — a counting permit pool (semaphore).
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

    // Acquire one permit. While waiting, the mutex is released,
    // so the worker currently holding a permit remains able to
    // release it (R5).
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Release one permit and wake a waiting worker.
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

// Role: worker body shared by w1 and w2.
// Each worker acquires the permit `times` times and, on every
// path through the loop, releases it exactly as many times (R4).
fn worker(_name: &'static str, s: Arc<Semaphore>, times: usize) {
    for _ in 0..times {
        s.acquire(); // hold the permit while doing the work (R3)
        // ... perform work while holding the permit ...
        s.release(); // always released before finishing this round
    }
    // All acquired permits have been released on every path.
}

fn main() {
    // Supervising task: launches w1 and w2 and waits for both (R1).
    // The permit pool begins with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);

    let h1 = thread::spawn(move || worker("w1", s_w1, 3));
    let h2 = thread::spawn(move || worker("w2", s_w2, 3));

    // Wait for both workers. Every interleaving terminates (R6):
    // a waiting worker blocks on the Condvar (releasing the mutex),
    // so the holder can always release and let it proceed.
    h1.join().unwrap();
    h2.join().unwrap();

    // Exactly one line of output, then exit (R7).
    println!("DONE done=1");
}
