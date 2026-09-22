use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Counting semaphore implemented with Mutex + Condvar (std only).
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Semaphore {
            count: Mutex::new(n),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cv.notify_one();
    }
}

fn main() {
    // Shared locks (mutexes) named a and b.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Counting permits named sa and sb.
    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));

    // Shared variable flag.
    let flag = Arc::new(Mutex::new(0u32));

    // Handles for the two workers.
    let mut handles = Vec::new();

    // Worker a.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);

        handles.push(thread::spawn(move || {
            // Take first lock a.
            let ga = a.lock().unwrap();
            // Signal that a has taken its first lock.
            sa.release();
            // Wait until b has taken its first lock.
            sb.acquire();
            // Now take second lock b.
            let gb = b.lock().unwrap();

            // Critical section: hold both locks.
            {
                let mut f = flag.lock().unwrap();
                *f |= 1;
            }

            // Release both locks before finishing.
            drop(gb);
            drop(ga);
        }));
    }

    // Worker b.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);

        handles.push(thread::spawn(move || {
            // Take first lock b.
            let gb = b.lock().unwrap();
            // Signal that b has taken its first lock.
            sb.release();
            // Wait until a has taken its first lock.
            sa.acquire();
            // Now take second lock a.
            let ga = a.lock().unwrap();

            // Critical section: hold both locks.
            {
                let mut f = flag.lock().unwrap();
                *f |= 2;
            }

            // Release both locks before finishing.
            drop(ga);
            drop(gb);
        }));
    }

    // Bystander task: keeps making progress without ever finishing on its own.
    {
        let flag = Arc::clone(&flag);
        thread::spawn(move || loop {
            let mut f = flag.lock().unwrap();
            // Make progress: toggle a bit that does not affect the final result.
            *f ^= 0;
            drop(f);
            // Yield to avoid starving others; still keeps running.
            thread::yield_now();
        });
    }

    // Main waits only for the two workers.
    for h in handles {
        h.join().unwrap();
    }

    let f = flag.lock().unwrap();
    let a_val = *f & 1;
    let b_val = (*f >> 1) & 1;
    println!("DONE a={} b={}", a_val, b_val);
}
