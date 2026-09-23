use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<u32>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: u32) -> Self {
        Self {
            permits: Mutex::new(count),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        *self.permits.lock().unwrap() += 1;
        self.cv.notify_one();
    }
}

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let sem_a = Arc::new(Semaphore::new(0));
    let sem_b = Arc::new(Semaphore::new(0));

    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_a = thread::spawn(move || {
        let ga = ma.lock().unwrap();
        sa.release();
        sb.acquire();
        let gb = mb.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_b = thread::spawn(move || {
        let gb = mb2.lock().unwrap();
        sb2.release();
        sa2.acquire();
        let ga = ma2.lock().unwrap();
        drop(ga);
        drop(gb);
    });

    let progress = thread::spawn(move || loop {
        thread::park();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    progress.thread().unpark();
    progress.join().unwrap();

    println!("DONE a=1 b=1");
}
