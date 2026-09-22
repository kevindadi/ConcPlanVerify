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
    s.acquire();
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    s.release();
    s.acquire();
    s.release();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));
    let mut done: i32 = 0;

    let h1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w2(s))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
}
