mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    cvar: Condvar,
}

struct Permit<'a> {
    sem: &'a Semaphore,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            cvar: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cvar.wait(permits).unwrap();
        }
        *permits -= 1;
        Permit { sem: self }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cvar.notify_one();
    }
}

impl<'a> Drop for Permit<'a> {
    fn drop(&mut self) {
        self.sem.release();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..3 {
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

    let handle1 = cir_trace::spawn("w1#1136", {
        let s = Arc::clone(&s);
        move || w1(s)
    });

    let handle2 = cir_trace::spawn("w2#1233", {
        let s = Arc::clone(&s);
        move || w2(s)
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
