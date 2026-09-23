mod concir_sync {
    use std::sync::{Arc, Condvar, Mutex};

    pub struct Semaphore {
        count: Mutex<usize>,
        cv: Condvar,
    }

    impl Semaphore {
        pub fn new(n: usize) -> Arc<Semaphore> {
            Arc::new(Semaphore {
                count: Mutex::new(n),
                cv: Condvar::new(),
            })
        }

        pub fn acquire(&self) -> Permit<'_> {
            let mut count = self.count.lock().unwrap();
            while *count == 0 {
                count = self.cv.wait(count).unwrap();
            }
            *count -= 1;
            Permit { sem: self }
        }

        pub fn try_acquire(&self) -> Option<Permit<'_>> {
            let mut count = self.count.lock().unwrap();
            if *count == 0 {
                None
            } else {
                *count -= 1;
                Some(Permit { sem: self })
            }
        }

        pub fn release(&self) {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.cv.notify_one();
        }
    }

    pub struct Permit<'a> {
        sem: &'a Semaphore,
    }

    impl<'a> Drop for Permit<'a> {
        fn drop(&mut self) {
            self.sem.release();
        }
    }
}

use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    // work
    drop(_permit);
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    // work
    drop(_permit);
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let t1 = thread::spawn(move || w1(s1));
    let t2 = thread::spawn(move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE permits=1");
}
