use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Condvar, Mutex,
};
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
        let mut permits = self
            .semaphore
            .permits
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        *permits += 1;
        self.semaphore.available.notify_one();
    }
}

fn w1(s: Arc<Semaphore>, work: Arc<AtomicUsize>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        work.fetch_add(1, Ordering::Relaxed);
    }
}

fn w2(s: Arc<Semaphore>, work: Arc<AtomicUsize>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        work.fetch_add(1, Ordering::Relaxed);
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1));
    let work = Arc::new(AtomicUsize::new(0));

    let s1 = Arc::clone(&s);
    let work1 = Arc::clone(&work);
    let t1 = thread::spawn(move || w1(s1, work1));

    let s2 = Arc::clone(&s);
    let work2 = Arc::clone(&work);
    let t2 = thread::spawn(move || w2(s2, work2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = 1;
    println!("DONE done={done}");
}
