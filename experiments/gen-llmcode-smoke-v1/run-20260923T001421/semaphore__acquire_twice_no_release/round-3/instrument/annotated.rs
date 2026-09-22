mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            count: Mutex::new(initial),
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

fn w2(s: Arc<Semaphore>, done: Arc<AtomicI32>) {
    s.acquire(1);
    s.release(1);
    s.acquire(1);
    s.release(1);
    let d = done.load(Ordering::SeqCst);
    let d2 = d + 1;
    done.store(d2, Ordering::SeqCst);
}

fn w1(s: Arc<Semaphore>, done: Arc<AtomicI32>) {
    let s1 = Arc::clone(&s);
    let d1 = Arc::clone(&done);
    let w1_handle = cir_trace::spawn("w1_handle", move || w2(s1, d1));

    let s2 = Arc::clone(&s);
    let d2 = Arc::clone(&done);
    let w2_handle = cir_trace::spawn("w2_handle", move || w2(s2, d2));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(AtomicI32::new(0));

    let s_sup = Arc::clone(&s);
    let done_sup = Arc::clone(&done);
    let sup = cir_trace::spawn("sup", move || w1(s_sup, done_sup));

    sup.join().unwrap();

    println!("DONE done={}", done.load(Ordering::SeqCst));
 cir_trace::finish();}
