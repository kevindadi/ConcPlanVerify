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

        pub fn acquire(&self) -> SemaphorePermit<'_> {
            let mut count = self.count.lock().unwrap();
            while *count == 0 {
                count = self.cv.wait(count).unwrap();
            }
            *count -= 1;
            SemaphorePermit { sem: self }
        }

        pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
            let mut count = self.count.lock().unwrap();
            if *count == 0 {
                None
            } else {
                *count -= 1;
                Some(SemaphorePermit { sem: self })
            }
        }

        pub fn release(&self) {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.cv.notify_one();
        }
    }

    pub struct SemaphorePermit<'a> {
        sem: &'a Semaphore,
    }

    impl<'a> Drop for SemaphorePermit<'a> {
        fn drop(&mut self) {
            self.sem.release();
        }
    }
}

fn worker() {}

fn main() {
    let h1 = std::thread::spawn(worker);
    h1.join().unwrap();
    let h2 = std::thread::spawn(worker);
    h2.join().unwrap();
    println!("DONE done=1");
}
