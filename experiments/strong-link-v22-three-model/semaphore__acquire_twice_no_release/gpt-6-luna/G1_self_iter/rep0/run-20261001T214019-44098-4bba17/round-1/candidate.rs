use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut permits = self.permits.lock().unwrap_or_else(|e| e.into_inner());

        while *permits == 0 {
            permits = self
                .available
                .wait(permits)
                .unwrap_or_else(|e| e.into_inner());
        }

        *permits -= 1;
        Permit { semaphore: self }
    }
}

struct Permit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        {
            let mut permits: MutexGuard<'_, usize> = self
                .semaphore
                .permits
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *permits += 1;
        }
        self.semaphore.available.notify_one();
    }
}

fn work() {
    thread::sleep(Duration::from_millis(1));
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        work();
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        work();
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let t1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let t2 = thread::spawn(move || w2(s2));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    println!("DONE done=1");
}
