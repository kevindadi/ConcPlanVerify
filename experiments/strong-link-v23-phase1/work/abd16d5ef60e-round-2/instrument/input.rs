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
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cond.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
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

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let t1 = thread::spawn(move || w1(s1));
    let t2 = thread::spawn(move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
