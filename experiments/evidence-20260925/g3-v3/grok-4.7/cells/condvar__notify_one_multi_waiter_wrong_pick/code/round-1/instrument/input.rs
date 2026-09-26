use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<i32>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: i32) -> Self {
        Semaphore {
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
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cv.notify_one();
    }
}

struct Shared {
    m: Mutex<()>,
    cv: Condvar,
    g12: Semaphore,
    gN: Semaphore,
}

fn w1(shared: &Shared) {
    let guard = shared.m.lock().unwrap();
    shared.g12.release();
    let guard = shared.cv.wait(guard).unwrap();
    drop(guard);
    shared.gN.release();
}

fn w2(shared: &Shared) {
    let guard = shared.m.lock().unwrap();
    shared.g12.release();
    let guard = shared.cv.wait(guard).unwrap();
    drop(guard);
    shared.gN.release();
}

fn notifier(shared: &Shared) {
    shared.g12.acquire();
    shared.g12.acquire();
    let guard = shared.m.lock().unwrap();
    shared.cv.notify_one();
    shared.cv.notify_one();
    drop(guard);
    shared.gN.acquire();
    shared.gN.acquire();
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        cv: Condvar::new(),
        g12: Semaphore::new(0),
        gN: Semaphore::new(0),
    });

    let s1 = Arc::clone(&shared);
    let h1 = thread::spawn(move || w1(&s1));

    let s2 = Arc::clone(&shared);
    let h2 = thread::spawn(move || w2(&s2));

    let sn = Arc::clone(&shared);
    let hn = thread::spawn(move || notifier(&sn));

    h1.join().unwrap();
    h2.join().unwrap();
    hn.join().unwrap();

    println!("DONE waiters=0");
}
