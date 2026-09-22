use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A counting semaphore built from a Mutex + Condvar.
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    /// Block until a permit is available, then take it.
    fn acquire(&self) {
        let mut guard = self.permits.lock().unwrap();
        while *guard == 0 {
            // Waiting releases the mutex, so a holder can always
            // reacquire it to release its permit.
            guard = self.available.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    /// Return a permit and wake one waiting worker.
    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
        self.available.notify_one();
    }
}

/// Worker body: acquire one permit, do its work, release the permit.
fn worker(s: &Semaphore) {
    s.acquire();

    // Perform the work while holding the permit.
    let mut acc: u64 = 0;
    for i in 0..1024u64 {
        acc = acc.wrapping_add(i);
    }
    std::hint::black_box(acc);

    s.release();
}

fn main() {
    // Shared counting permit pool `s` starting with exactly two permits.
    let s = Arc::new(Semaphore::new(2));

    // Supervising task launches the three workers.
    let s1 = Arc::clone(&s);
    let w1 = thread::spawn(move || worker(&s1));

    let s2 = Arc::clone(&s);
    let w2 = thread::spawn(move || worker(&s2));

    let s3 = Arc::clone(&s);
    let w3 = thread::spawn(move || worker(&s3));

    // Wait for all workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
