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
        let mut guard = self.count.lock().unwrap();
        while *guard == 0 {
            guard = self.cond.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.count.lock().unwrap();
        *guard += 1;
        self.cond.notify_one();
    }
}

fn w1(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
}

fn w3(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new(0i32));

    let h1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let h2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h3 = thread::spawn({
        let s = Arc::clone(&s);
        move || w3(s)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    *done.lock().unwrap() = 1;

    println!("DONE done={}", *done.lock().unwrap());
}
