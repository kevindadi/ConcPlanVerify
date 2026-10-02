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

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }
}

fn work(s: &Semaphore) {
    s.acquire();
    thread::yield_now();
    s.release();
}

fn w1(s: Arc<Semaphore>) {
    work(&s);
}

fn w2(s: Arc<Semaphore>) {
    work(&s);
}

fn w3(s: Arc<Semaphore>) {
    work(&s);
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(2));

    let handles = [
        cir_trace::spawn("w1#957", {
            let s = Arc::clone(&s);
            move || w1(s)
        }),
        cir_trace::spawn("w2#1055", {
            let s = Arc::clone(&s);
            move || w2(s)
        }),
        cir_trace::spawn("w3#1153", {
            let s = Arc::clone(&s);
            move || w3(s)
        }),
    ];

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
