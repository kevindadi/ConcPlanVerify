mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

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

fn w1(s: Arc<Semaphore>) {
    s.acquire();
    let _work = ();
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    let _work = ();
    s.release();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let worker1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#921", move || w1(s))
    };
    let worker2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#1022", move || w2(s))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE permits={}", *s.permits.lock().unwrap());
 cir_trace::finish();}
