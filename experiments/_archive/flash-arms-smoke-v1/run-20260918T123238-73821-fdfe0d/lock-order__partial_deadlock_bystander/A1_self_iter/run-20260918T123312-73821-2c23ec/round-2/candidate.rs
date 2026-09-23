use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Semaphore {
    mutex: Mutex<usize>,
    condvar: std::sync::Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            mutex: Mutex::new(permits),
            condvar: std::sync::Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.mutex.lock().unwrap();
        while *permits == 0 {
            permits = self.condvar.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.mutex.lock().unwrap();
        *permits += 1;
        self.condvar.notify_one();
    }
}

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let sem = Arc::new(Semaphore::new(0));

    let m1a = Arc::clone(&m1);
    let m2a = Arc::clone(&m2);
    let sem_a = Arc::clone(&sem);

    let a = thread::spawn(move || {
        let _g1 = m1a.lock().unwrap();
        sem_a.release();
        let _g2 = m2a.lock().unwrap();
    });

    let m1b = Arc::clone(&m1);
    let m2b = Arc::clone(&m2);
    let sem_b = Arc::clone(&sem);

    let b = thread::spawn(move || {
        sem_b.acquire();
        let _g2 = m2b.lock().unwrap();
        let _g1 = m1b.lock().unwrap();
    });

    let bystander = thread::spawn(|| loop {
        thread::sleep(Duration::from_millis(1));
    });

    a.join().unwrap();
    b.join().unwrap();

    // Bystander runs forever; detach it by not joining.
    drop(bystander);
}
