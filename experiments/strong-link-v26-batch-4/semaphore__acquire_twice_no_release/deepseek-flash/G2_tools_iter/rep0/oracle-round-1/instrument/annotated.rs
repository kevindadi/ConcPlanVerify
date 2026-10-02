mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    cvar: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            cvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cvar.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cvar.notify_one();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        s.acquire();
        std::thread::yield_now();
        s.release();
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        s.acquire();
        std::thread::yield_now();
        s.release();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let supervisor = {
        let s = Arc::clone(&s);
        cir_trace::spawn("clone#1022", move || {
            let w1_handle = {
                let s = Arc::clone(&s);
                cir_trace::spawn("w1#1132", move || w1(s))
            };

            let w2_handle = {
                let s = Arc::clone(&s);
                cir_trace::spawn("w2#1263", move || w2(s))
            };

            w1_handle.join().unwrap();
            w2_handle.join().unwrap();
        })
    };

    supervisor.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
