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

    fn acquire(&self, n: usize) {
        let mut count = self.count.lock().unwrap();
        while *count < n {
            count = self.cond.wait(count).unwrap();
        }
        *count -= n;
    }

    fn release(&self, n: usize) {
        let mut count = self.count.lock().unwrap();
        *count += n;
        self.cond.notify_all();
    }
}

fn w2(s: Arc<Semaphore>) {
    s.acquire(1);
    s.release(1);
}

fn w1(s: Arc<Semaphore>, done: Arc<Mutex<bool>>) {
    let h1 = cir_trace::spawn("h1", {
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h2 = cir_trace::spawn("h2", {
        let s = Arc::clone(&s);
        move || w2(s)
    });
    h1.join().unwrap();
    h2.join().unwrap();
    *done.lock().unwrap() = true;
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new_named("done_mutex0", false));

    let sup = cir_trace::spawn("sup", {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w1(s, done)
    });
    sup.join().unwrap();

    let _ = *done.lock().unwrap();
    println!("DONE permits=1");
 cir_trace::finish();}
