use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A counting semaphore with a Mutex + Condvar.
struct Semaphore {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            condvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.condvar.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.condvar.notify_one();
    }
}

fn worker(s: Arc<Semaphore>, name: &'static str) {
    // Acquire the permit, do work, release it.
    s.acquire();
    println!("{} working", name);
    s.release();

    // Acquire again to demonstrate multiple acquisitions.
    s.acquire();
    println!("{} working again", name);
    s.release();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = thread::spawn(move || worker(s1, "w1"));
    let w2 = thread::spawn(move || worker(s2, "w2"));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
