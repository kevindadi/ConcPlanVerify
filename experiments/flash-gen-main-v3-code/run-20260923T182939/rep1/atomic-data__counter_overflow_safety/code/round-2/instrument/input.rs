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

use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: &Mutex<Shared>) {
    let mut guard = m.lock().unwrap();
    let tmp = guard.c;
    if tmp < 1 {
        guard.c = tmp + 1;
    }
}

fn w2(m: &Mutex<Shared>) {
    let mut guard = m.lock().unwrap();
    let tmp = guard.c;
    if tmp < 1 {
        guard.c = tmp + 1;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let h1 = thread::spawn(move || {
        w1(&m1);
    });

    let m2 = Arc::clone(&m);
    let h2 = thread::spawn(move || {
        w2(&m2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = {
        let guard = m.lock().unwrap();
        guard.c
    };

    println!("DONE done={}", done);
}
