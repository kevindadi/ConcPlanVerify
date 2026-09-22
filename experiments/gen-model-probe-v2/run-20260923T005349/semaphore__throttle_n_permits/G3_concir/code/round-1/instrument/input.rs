use std::sync::{Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            condvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut guard = self.count.lock().unwrap();
        while *guard == 0 {
            guard = self.condvar.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.count.lock().unwrap();
        *guard += 1;
        self.condvar.notify_one();
    }
}

fn w1(s: &Semaphore) {
    s.acquire();
    // perform work while holding the permit
    s.release();
}

fn w2(s: &Semaphore) {
    s.acquire();
    // perform work while holding the permit
    s.release();
}

fn w3(s: &Semaphore) {
    s.acquire();
    // perform work while holding the permit
    s.release();
}

fn main() {
    let s = Semaphore::new(2);
    #[allow(unused_assignments)]
    let mut done = 0;

    thread::scope(|scope| {
        scope.spawn(|| w1(&s));
        scope.spawn(|| w2(&s));
        scope.spawn(|| w3(&s));
    });

    done = 1;
    println!("DONE done={}", done);
}
