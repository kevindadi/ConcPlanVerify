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

    fn acquire(&self) -> Permit<'_> {
        let mut permits = self.permits.lock().unwrap();

        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }

        *permits -= 1;
        Permit { semaphore: self }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }
}

struct Permit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

fn do_work() {
    let mut work = 0;
    work += 1;
    std::hint::black_box(work);
}

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    do_work();
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    do_work();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let s_for_w1 = Arc::clone(&s);
    let s_for_w2 = Arc::clone(&s);

    let t1 = cir_trace::spawn("w1#1232", move || w1(s_for_w1));
    let t2 = cir_trace::spawn("w2#1282", move || w2(s_for_w2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
