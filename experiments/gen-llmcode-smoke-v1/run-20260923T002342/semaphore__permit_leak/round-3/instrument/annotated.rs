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
        let mut c = self.count.lock().unwrap();
        while *c < n {
            c = self.cond.wait(c).unwrap();
        }
        *c -= n;
    }

    fn release(&self, n: usize) {
        let mut c = self.count.lock().unwrap();
        *c += n;
        self.cond.notify_all();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new_named("done_mutex0", false));

    let s1 = Arc::clone(&s);
    let done1 = Arc::clone(&done);
    let sup = cir_trace::spawn("sup", move || {
        w1(s1, done1);
    });

    sup.join().unwrap();

    let d = done.lock().unwrap();
    if *d {
        println!("DONE permits=1");
    }
 cir_trace::finish();}

fn w1(s: Arc<Semaphore>, done: Arc<Mutex<bool>>) {
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let h1 = cir_trace::spawn("h1", move || {
        w2(s1);
    });
    let h2 = cir_trace::spawn("h2", move || {
        w2(s2);
    });
    h1.join().unwrap();
    h2.join().unwrap();

    let mut d = done.lock().unwrap();
    *d = true;
}

fn w2(s: Arc<Semaphore>) {
    s.acquire(1);
    s.release(1);
}
