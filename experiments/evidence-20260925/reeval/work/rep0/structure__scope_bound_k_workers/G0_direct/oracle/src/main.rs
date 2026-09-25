mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            condvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.condvar.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.condvar.notify_one();
    }
}

fn activation(s: Arc<Semaphore>, done: Arc<Mutex<usize>>) {
    s.acquire();
    // critical section: hold the single permit while doing work
    let mut d = done.lock().unwrap();
    *d += 1;
    drop(d);
    s.release();
}

fn role(s: Arc<Semaphore>, done: Arc<Mutex<usize>>) {
    let mut handles = Vec::new();
    for _ in 0..2 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || activation(s, done)));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0usize));

    let mut roles = Vec::new();
    for _ in 0..3 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        roles.push(thread::spawn(move || role(s, done)));
    }

    for r in roles {
        r.join().unwrap();
    }

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
 cir_trace::finish();}
