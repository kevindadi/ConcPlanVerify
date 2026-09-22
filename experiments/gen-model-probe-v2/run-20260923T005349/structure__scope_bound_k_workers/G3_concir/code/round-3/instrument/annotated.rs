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
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
}

fn w3(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0i32));

    let h1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("h1", move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("h2", move || w2(s))
    };
    let h3 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("h3", move || w3(s))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    {
        let mut d = done.lock().unwrap();
        *d = 1;
    }

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
 cir_trace::finish();}
