use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Permit {
    available: Mutex<bool>,
    cv: Condvar,
}

impl Permit {
    fn new() -> Self {
        Permit {
            available: Mutex::new(true),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut avail = self.available.lock().unwrap();
        while !*avail {
            avail = self.cv.wait(avail).unwrap();
        }
        *avail = false;
    }

    fn release(&self) {
        let mut avail = self.available.lock().unwrap();
        *avail = true;
        self.cv.notify_one();
    }
}

fn main() {
    let permit = Arc::new(Permit::new());
    let done = Arc::new(Mutex::new(0u32));

    let mut handles = Vec::new();

    for _role in 0..3 {
        for _activation in 0..2 {
            let permit = Arc::clone(&permit);
            let done = Arc::clone(&done);
            let handle = thread::spawn(move || {
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
    println!("DONE done={}", *d);
}
