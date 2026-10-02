mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::hint::black_box;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
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
        let mut permits = self
            .permits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        while *permits == 0 {
            permits = self
                .available
                .wait(permits)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }

        *permits -= 1;
        Permit { s: self }
    }
}

struct Permit<'a> {
    s: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut permits = self
            .s
            .permits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *permits += 1;
        drop(permits);
        self.s.available.notify_one();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        let mut total = 0u64;
        for i in 0..100_000u64 {
            total = total.wrapping_add(i);
        }
        black_box(total);
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        let mut total = 0u64;
        for i in 0..100_000u64 {
            total = total.wrapping_add(i);
        }
        black_box(total);
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = AtomicBool::new(false);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1#1762", move || w1(s1));
    let h2 = cir_trace::spawn("w2#1806", move || w2(s2));

    let r1 = h1.join();
    let r2 = h2.join();
    assert!(r1.is_ok() && r2.is_ok());

    { let __cpv = done.store(true, Ordering::Relaxed); cir_trace::record_value("done#1666", (done.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    println!("DONE done={}", if done.load(Ordering::Relaxed) { 1 } else { 0 });
 cir_trace::finish();}
