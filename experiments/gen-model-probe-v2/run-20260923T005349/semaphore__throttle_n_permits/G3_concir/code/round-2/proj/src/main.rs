mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

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
        let mut guard = self.count.lock().unwrap();
        while *guard == 0 {
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

fn main() { cir_trace::init();
    let s = Semaphore::new(2);
    let mut done: i32 = 0;

    let w1 = || {
        s.acquire();
        s.release();
    };
    let w2 = || {
        s.acquire();
        s.release();
    };
    let w3 = || {
        s.acquire();
        s.release();
    };

    thread::scope(|scope| {
        scope.spawn(w1);
        scope.spawn(w2);
        scope.spawn(w3);
    });

    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
