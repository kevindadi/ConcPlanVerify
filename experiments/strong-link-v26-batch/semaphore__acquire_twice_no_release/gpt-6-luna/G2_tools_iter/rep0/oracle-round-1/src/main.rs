mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering};

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

    fn acquire(self: &Arc<Self>) -> Permit {
        let mut permits = self
            .permits
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        while *permits == 0 {
            permits = self
                .available
                .wait(permits)
                .unwrap_or_else(PoisonError::into_inner);
        }

        *permits -= 1;
        Permit {
            semaphore: Arc::clone(self),
        }
    }

    fn release(&self) {
        let mut permits = self
            .permits
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *permits += 1;
        drop(permits);
        self.available.notify_one();
    }
}

struct Permit {
    semaphore: Arc<Semaphore>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.semaphore.release();
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

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let work = Arc::new(AtomicUsize::new(0));

    let s1 = Arc::clone(&s);
    let work1 = Arc::clone(&work);
    let t1 = cir_trace::spawn("w1#1672", move || w1(s1, work1));

    let s2 = Arc::clone(&s);
    let work2 = Arc::clone(&work);
    let t2 = cir_trace::spawn("w2#1793", move || w2(s2, work2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
