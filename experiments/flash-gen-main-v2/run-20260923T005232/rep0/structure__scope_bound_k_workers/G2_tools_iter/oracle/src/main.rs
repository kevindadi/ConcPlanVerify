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

fn worker(s: Arc<Semaphore>, done: Arc<Mutex<usize>>) {
    s.acquire();
    *done.lock().unwrap() += 1;
    s.release();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0usize));

    let mut handles = Vec::new();

    // Role w1: up to two activations.
    for _ in 0..2 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || worker(s, done)));
    }

    // Role w2: up to two activations.
    for _ in 0..2 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || worker(s, done)));
    }

    // Role w3: up to two activations.
    for _ in 0..2 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || worker(s, done)));
    }

    for h in handles {
        h.join().unwrap();
    }

    let d = *done.lock().unwrap();
    println!("DONE done={}", d);
 cir_trace::finish();}
