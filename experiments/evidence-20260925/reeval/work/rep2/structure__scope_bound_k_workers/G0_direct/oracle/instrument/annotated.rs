mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cond.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cond.notify_one();
    }
}

fn activation(s: &Semaphore) {
    s.acquire();
    // critical section: hold the single permit while doing work
    s.release();
}

fn worker(s: Arc<Semaphore>) {
    // each role may have up to two activations running at once
    let mut handles = Vec::new();
    for _ in 0..2 {
        let s2 = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            activation(&s2);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let mut roles = Vec::new();
    for _ in 0..3 {
        let s2 = Arc::clone(&s);
        roles.push(thread::spawn(move || {
            worker(s2);
        }));
    }

    for r in roles {
        r.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
