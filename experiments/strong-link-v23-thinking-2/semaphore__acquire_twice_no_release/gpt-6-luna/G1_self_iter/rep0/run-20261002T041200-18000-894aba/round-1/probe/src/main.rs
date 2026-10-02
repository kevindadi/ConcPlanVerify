use std::sync::{Arc, Condvar, Mutex};
use std::thread;

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
        let mut permits = self.permits.lock().unwrap();

        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }

        *permits -= 1;
        Permit { semaphore: self }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        drop(permits);
        self.available.notify_one();
    }
}

struct Permit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        thread::yield_now();
        drop(permit);
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        thread::yield_now();
        drop(permit);
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let t1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let t2 = thread::spawn(move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
