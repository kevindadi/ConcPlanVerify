mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

struct Semaphore {
    count: Mutex<i64>,
    cond: Condvar,
}

impl Semaphore {
    fn new(permits: i64) -> Self {
        Semaphore {
            count: Mutex::new(permits),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut guard = self.count.lock().unwrap();
        while *guard <= 0 {
            guard = self.cond.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.count.lock().unwrap();
        *guard += 1;
        self.cond.notify_one();
    }
}

fn w1(s: &Semaphore) {
    s.acquire();
    s.release();
}

fn w2(s: &Semaphore) {
    s.acquire();
    s.release();
}

fn w3(s: &Semaphore) {
    s.acquire();
    s.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new(2);
    let mut done: i64 = 0;

    thread::scope(|scope| {
        scope.spawn(|| w1(&s));
        scope.spawn(|| w2(&s));
        scope.spawn(|| w3(&s));
    });

    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
