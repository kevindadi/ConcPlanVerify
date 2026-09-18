use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cond.notify_one();
    }
}

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let sem = Arc::new(Semaphore::new(0));

    let a_m1 = Arc::clone(&m1);
    let a_m2 = Arc::clone(&m2);
    let a_sem = Arc::clone(&sem);

    let b_m1 = Arc::clone(&m1);
    let b_m2 = Arc::clone(&m2);
    let b_sem = Arc::clone(&sem);

    let stop = Arc::new(Mutex::new(false));

    let bystander_stop = Arc::clone(&stop);

    let a = thread::spawn(move || {
        let _g1 = a_m1.lock().unwrap();
        a_sem.release();
        let _g2 = a_m2.lock().unwrap();
    });

    let b = thread::spawn(move || {
        let _g1 = b_m1.lock().unwrap();
        b_sem.acquire();
        let _g2 = b_m2.lock().unwrap();
    });

    let bystander = thread::spawn(move || loop {
        let s = bystander_stop.lock().unwrap();
        if *s {
            break;
        }
    });

    a.join().unwrap();
    b.join().unwrap();

    {
        let mut s = stop.lock().unwrap();
        *s = true;
    }

    bystander.join().unwrap();
}
