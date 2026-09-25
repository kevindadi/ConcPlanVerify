use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            count: Mutex::new(permits),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cond.notify_one();
    }
}

fn activation(s: &Semaphore, done: &Mutex<usize>) {
    s.acquire();
    // critical section: hold the single permit
    let mut d = done.lock().unwrap();
    *d += 1;
    drop(d);
    s.release();
}

fn worker(s: Arc<Semaphore>, done: Arc<Mutex<usize>>) {
    // each role may have up to two activations running at once
    let mut handles = Vec::new();
    for _ in 0..2 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || {
            activation(&s, &done);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new(0));

    let mut workers = Vec::new();
    for _ in 0..3 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        workers.push(thread::spawn(move || {
            worker(s, done);
        }));
    }

    for w in workers {
        w.join().unwrap();
    }

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
}
