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
            // wait releases the mutex, allowing the permit holder to release.
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        drop(permits);
        self.available.notify_one();
    }
}

fn work(s: &Semaphore) {
    s.acquire();

    let mut total = 0_u64;
    for i in 0..1_000 {
        total += i;
    }
    assert_eq!(total, 499_500);

    s.release();
}

fn w1(s: Arc<Semaphore>) {
    work(&s);
}

fn w2(s: Arc<Semaphore>) {
    work(&s);
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let first = cir_trace::spawn("w1#1072", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let second = cir_trace::spawn("w2#1172", {
        let s = Arc::clone(&s);
        move || w2(s)
    });

    first.join().unwrap();
    second.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
