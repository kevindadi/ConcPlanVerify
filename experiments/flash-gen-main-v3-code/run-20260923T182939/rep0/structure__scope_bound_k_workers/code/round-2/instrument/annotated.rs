mod cir_trace;
mod concir_sync;
use cir_trace::sync::{Mutex, Condvar};
mod concir_sync {
    use std::sync::{Arc};

    pub struct Semaphore {
        count: Mutex<usize>,
        cv: Condvar,
    }

    pub struct Permit {
        sem: Arc<Semaphore>,
    }

    impl Drop for Permit {
        fn drop(&mut self) {
            self.sem.release();
        }
    }

    impl Semaphore {
        pub fn new(n: usize) -> Arc<Semaphore> {
            Arc::new(Semaphore {
                count: Mutex::new(n),
                cv: Condvar::new(),
            })
        }

        pub fn acquire(self: &Arc<Self>) -> Permit {
            let mut count = self.count.lock().unwrap();
            while *count == 0 {
                count = self.cv.wait(count).unwrap();
            }
            *count -= 1;
            Permit { sem: Arc::clone(self) }
        }

        pub fn try_acquire(self: &Arc<Self>) -> Option<Permit> {
            let mut count = self.count.lock().unwrap();
            if *count == 0 {
                None
            } else {
                *count -= 1;
                Some(Permit { sem: Arc::clone(self) })
            }
        }

        pub fn release(&self) {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.cv.notify_one();
        }
    }
}

use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn w3(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn main() { cir_trace::init();
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);

    let h1 = cir_trace::spawn("w1", move || w1(s1));
    let h2 = cir_trace::spawn("w2", move || w2(s2));
    let h3 = cir_trace::spawn("w3", move || w3(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
