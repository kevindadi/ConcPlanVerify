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

        pub fn acquire(self: &Arc<Self>) -> SemaphorePermit {
            let mut count = self.count.lock().unwrap();
            while *count == 0 {
                count = self.cv.wait(count).unwrap();
            }
            *count -= 1;
            SemaphorePermit {
                sem: Arc::clone(self),
            }
        }

        pub fn try_acquire(self: &Arc<Self>) -> Option<SemaphorePermit> {
            let mut count = self.count.lock().unwrap();
            if *count == 0 {
                None
            } else {
                *count -= 1;
                Some(SemaphorePermit {
                    sem: Arc::clone(self),
                })
            }
        }

        pub fn release(self: &Arc<Self>) {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.cv.notify_one();
        }
    }

    pub struct SemaphorePermit {
        sem: Arc<Semaphore>,
    }

    impl Drop for SemaphorePermit {
        fn drop(&mut self) {
            self.sem.release();
        }
    }
}

use std::sync::Arc;
use std::thread;

use concir_sync::Semaphore;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE permits=1");
}
