use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A counting semaphore implemented from a mutex and a condition variable.
/// A thread waiting in `acquire` blocks on the condvar, which releases the
/// mutex, so the permit holder always remains able to call `release`.
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

    fn acquire(&self) {
        let mut guard = self.permits.lock().unwrap();
        while *guard == 0 {
            guard = self.available.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
        self.available.notify_one();
    }
}

/// Worker body shared by w1 and w2. Each worker acquires the permit more
/// than once, and every acquire is matched by exactly one release on every
/// path before the worker finishes.
fn worker(
    name: &'static str,
    s: Arc<Semaphore>,
    in_section: Arc<AtomicBool>,
    work: Arc<AtomicU64>,
) {
    for _ in 0..2 {
        s.acquire();

        // Critical section: the single permit guarantees that the two
        // workers never perform their work at the same time.
        let already_inside = in_section.swap(true, Ordering::SeqCst);
        assert!(
            !already_inside,
            "{} entered the critical section while another worker held the permit",
            name
        );
        work.fetch_add(1, Ordering::SeqCst);
        in_section.store(false, Ordering::SeqCst);

        s.release();
    }
}

fn main() {
    // Shared counting permit pool `s`, starting with exactly one permit.
    let s = Arc::new(Semaphore::new(1));
    let in_section = Arc::new(AtomicBool::new(false));
    let work = Arc::new(AtomicU64::new(0));

    // Supervising task: launch worker threads w1 and w2.
    let w1 = {
        let s = Arc::clone(&s);
        let in_section = Arc::clone(&in_section);
        let work = Arc::clone(&work);
        thread::Builder::new()
            .name("w1".to_string())
            .spawn(move || worker("w1", s, in_section, work))
            .unwrap()
    };

    let w2 = {
        let s = Arc::clone(&s);
        let in_section = Arc::clone(&in_section);
        let work = Arc::clone(&work);
        thread::Builder::new()
            .name("w2".to_string())
            .spawn(move || worker("w2", s, in_section, work))
            .unwrap()
    };

    // Wait for both workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();

    // Both workers completed all of their bounded critical sections.
    let done = if work.load(Ordering::SeqCst) == 4 { 1 } else { 0 };
    println!("DONE done={}", done);
}
