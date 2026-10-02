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

fn run_worker(s: Arc<Semaphore>) {
    s.acquire();
    let _work = ();
    s.release();
}

fn w1(s: Arc<Semaphore>) {
    run_worker(s);
}

fn w2(s: Arc<Semaphore>) {
    run_worker(s);
}

fn w3(s: Arc<Semaphore>) {
    run_worker(s);
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(2));

    let t1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#1004", move || w1(s))
    };
    let t2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#1095", move || w2(s))
    };
    let t3 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w3#1186", move || w3(s))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
