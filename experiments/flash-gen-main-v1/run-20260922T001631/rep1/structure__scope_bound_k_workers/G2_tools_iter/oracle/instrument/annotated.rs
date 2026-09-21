mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Permit {
    available: Mutex<bool>,
    condvar: Condvar,
}

impl Permit {
    fn new() -> Self {
        Permit {
            available: Mutex::new(true),
            condvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut available = self.available.lock().unwrap();
        while !*available {
            available = self.condvar.wait(available).unwrap();
        }
        *available = false;
    }

    fn release(&self) {
        let mut available = self.available.lock().unwrap();
        *available = true;
        self.condvar.notify_one();
    }
}

fn main() { cir_trace::init();
    let permit = Arc::new(Permit::new());
    let done = Arc::new(Mutex::new_named("done_mutex0", 0u32));

    let mut handles = Vec::new();

    for _role in 0..3 {
        for _activation in 0..2 {
            let permit = Arc::clone(&permit);
            let done = Arc::clone(&done);
            let handle = cir_trace::spawn("handle", move || {
                permit.acquire();
                // Do the work while holding the single permit.
                {
                    let mut d = done.lock().unwrap();
                    *d += 1;
                }
                permit.release();
            });
            handles.push(handle);
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let done_count = *done.lock().unwrap();
    println!("DONE done={}", done_count);
 cir_trace::finish();}
