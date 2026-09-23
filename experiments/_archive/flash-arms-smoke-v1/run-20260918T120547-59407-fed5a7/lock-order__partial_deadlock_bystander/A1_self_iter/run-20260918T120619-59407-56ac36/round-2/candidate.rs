use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Semaphore {
    mutex: Mutex<usize>,
    condvar: std::sync::Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            mutex: Mutex::new(count),
            condvar: std::sync::Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.mutex.lock().unwrap();
        while *count == 0 {
            count = self.condvar.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.mutex.lock().unwrap();
        *count += 1;
        self.condvar.notify_one();
    }
}

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    // sem_ab: A signals it has taken m1; B waits before taking m2.
    let sem_ab = Arc::new(Semaphore::new(0));
    // sem_ba: B signals it has taken m2; A waits before taking m2.
    let sem_ba = Arc::new(Semaphore::new(0));

    let m1a = Arc::clone(&m1);
    let m2a = Arc::clone(&m2);
    let sem_ab_a = Arc::clone(&sem_ab);
    let sem_ba_a = Arc::clone(&sem_ba);
    let a = thread::spawn(move || {
        let _g1 = m1a.lock().unwrap();
        sem_ab_a.release();
        sem_ba_a.acquire();
        let _g2 = m2a.lock().unwrap();
    });

    let m1b = Arc::clone(&m1);
    let m2b = Arc::clone(&m2);
    let sem_ab_b = Arc::clone(&sem_ab);
    let sem_ba_b = Arc::clone(&sem_ba);
    let b = thread::spawn(move || {
        sem_ab_b.acquire();
        let _g2 = m2b.lock().unwrap();
        sem_ba_b.release();
        let _g1 = m1b.lock().unwrap();
    });

    let bystander = thread::spawn(|| loop {
        thread::sleep(Duration::from_millis(1));
    });

    a.join().unwrap();
    b.join().unwrap();
    bystander.join().unwrap();
}
