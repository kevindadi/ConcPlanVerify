mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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
        Permit { s: self }
    }
}

struct Permit<'a> {
    s: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut permits = self.s.permits.lock().unwrap();
        *permits += 1;
        drop(permits);
        self.s.available.notify_one();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        thread::yield_now();
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        thread::yield_now();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let t1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#1150", move || w1(s))
    };
    let t2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#1241", move || w2(s))
    };

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
