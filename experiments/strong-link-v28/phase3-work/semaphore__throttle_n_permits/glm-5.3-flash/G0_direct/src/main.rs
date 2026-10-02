mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource: s (counting permit pool)
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            // Wait while a holder remains able to release (R5)
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

// Worker role: w1
fn w1(s: &Arc<Semaphore>) {
    s.acquire();
    // perform work
    s.release();
}

// Worker role: w2
fn w2(s: &Arc<Semaphore>) {
    s.acquire();
    // perform work
    s.release();
}

// Worker role: w3
fn w3(s: &Arc<Semaphore>) {
    s.acquire();
    // perform work
    s.release();
}

fn main() { cir_trace::init();
    // Supervising task: launches three workers and waits for all of them (R1)
    // Pool begins with exactly two permits (R2)
    let s = Arc::new(Semaphore::new(2));

    let s1 = Arc::clone(&s);
    let t1 = cir_trace::spawn("w1#1264", move || w1(&s1));
    let s2 = Arc::clone(&s);
    let t2 = cir_trace::spawn("w2#1338", move || w2(&s2));
    let s3 = Arc::clone(&s);
    let t3 = cir_trace::spawn("w3#1412", move || w3(&s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    // R7: print exactly this line and exit
    println!("DONE done=1");
 cir_trace::finish();}
