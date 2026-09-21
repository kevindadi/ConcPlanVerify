mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Permit {
    held: Mutex<bool>,
    cv: Condvar,
}

impl Permit {
    fn new() -> Self {
        Permit {
            held: Mutex::new(false),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut held = self.held.lock().unwrap();
        while *held {
            held = self.cv.wait(held).unwrap();
        }
        *held = true;
    }

    fn release(&self) {
        let mut held = self.held.lock().unwrap();
        *held = false;
        self.cv.notify_one();
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
                // critical section: hold the single permit
                let mut d = done.lock().unwrap();
                *d += 1;
                drop(d);
                permit.release();
            });
            handles.push(handle);
        }
    }

    for h in handles {
        h.join().unwrap();
    }

    let d = done.lock().unwrap();
    if *d == 6 {
        println!("DONE done=1");
    } else {
        println!("DONE done=0");
    }
 cir_trace::finish();}
