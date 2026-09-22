mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cvar: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            cvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut guard = self.count.lock().unwrap();
        while *guard == 0 {
            guard = self.cvar.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.count.lock().unwrap();
        *guard += 1;
        self.cvar.notify_one();
    }
}

fn w1(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
    s.acquire();
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
    s.acquire();
    s.release();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let mut done = 0;

    let s_for_w1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("h1", move || w1(s_for_w1));

    let s_for_w2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("h2", move || w2(s_for_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
