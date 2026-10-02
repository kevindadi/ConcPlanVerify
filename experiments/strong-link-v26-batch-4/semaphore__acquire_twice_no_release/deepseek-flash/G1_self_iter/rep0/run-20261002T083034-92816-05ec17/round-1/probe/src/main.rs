use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cond.wait(permits).unwrap();
        }
        *permits -= 1;
        Permit { s: self }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cond.notify_one();
    }
}

struct Permit<'a> {
    s: &'a Semaphore,
}

impl<'a> Drop for Permit<'a> {
    fn drop(&mut self) {
        self.s.release();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();

        // Work while holding the permit.
        let mut x = 0u64;
        for i in 0..1000 {
            x = x.wrapping_add(i);
        }
        let _ = x;
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();

        // Work while holding the permit.
        let mut y = 0u64;
        for i in 0..1000 {
            y = y.wrapping_add(i);
        }
        let _ = y;
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1_handle = thread::spawn(move || w1(s1));
    let w2_handle = thread::spawn(move || w2(s2));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
}
