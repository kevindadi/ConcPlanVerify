use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A counting semaphore with a single permit.
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

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let mut handles = Vec::new();

    for _ in 0..3 {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            // Each role may have up to two activations running at once.
            let mut activations = Vec::new();
            for _ in 0..2 {
                let s = Arc::clone(&s);
                activations.push(thread::spawn(move || {
                    s.acquire();
                    // Do work while holding the permit.
                    s.release();
                }));
            }
            for a in activations {
                a.join().unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
