mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
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
        Permit { s: self }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap_or_else(|e| e.into_inner());
        *permits += 1;
        self.available.notify_one();
    }
}

struct Permit<'a> {
    s: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.s.release();
    }
}

fn w1(s: Arc<Semaphore>, work: Arc<AtomicUsize>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#1394", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    }
}

fn w2(s: Arc<Semaphore>, work: Arc<AtomicUsize>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#1394", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let work = Arc::new(AtomicUsize::new(0));

    let s1 = Arc::clone(&s);
    let work1 = Arc::clone(&work);
    let h1 = cir_trace::spawn("w1#1503", move || w1(s1, work1));

    let s2 = Arc::clone(&s);
    let work2 = Arc::clone(&work);
    let h2 = cir_trace::spawn("w2#1619", move || w2(s2, work2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
