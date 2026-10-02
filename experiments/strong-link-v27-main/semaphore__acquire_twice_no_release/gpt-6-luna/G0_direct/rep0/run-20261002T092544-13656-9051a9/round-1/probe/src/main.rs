use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn acquire(s: &Arc<Self>) -> Permit {
        let mut permits = s.permits.lock().unwrap();
        while *permits == 0 {
            permits = s.available.wait(permits).unwrap();
        }
        *permits -= 1;
        drop(permits);

        Permit { s: Arc::clone(s) }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        drop(permits);
        self.available.notify_one();
    }
}

struct Permit {
    s: Arc<Semaphore>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.s.release();
    }
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = Semaphore::acquire(&s);
        thread::yield_now();
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = Semaphore::acquire(&s);
        thread::yield_now();
    }
}

fn main() {
    let s = Arc::new(Semaphore {
        permits: Mutex::new(1),
        available: Condvar::new(),
    });

    let worker1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w1(s))
    };
    let worker2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w2(s))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
}
