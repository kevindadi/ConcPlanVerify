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

    fn permits(&self) -> usize {
        *self.count.lock().unwrap()
    }
}

fn worker(s: Arc<Semaphore>, name: &'static str) {
    s.acquire();
    // Perform work while holding the permit.
    println!("{} working", name);
    s.release();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = cir_trace::spawn("w1", move || worker(s1, "w1"));
    let w2 = cir_trace::spawn("w2", move || worker(s2, "w2"));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits={}", s.permits());
 cir_trace::finish();}
