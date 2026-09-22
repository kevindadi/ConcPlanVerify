use std::sync::{Arc, Condvar, Mutex};
use std::thread;

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
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));

    let flag = Arc::new(Mutex::new(0u32));

    let mut handles = Vec::new();

    // Worker a: takes a first, then b (consistent order a -> b).
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);

        handles.push(thread::spawn(move || {
            let ga = a.lock().unwrap();
            sa.release();
            sb.acquire();
            let gb = b.lock().unwrap();

            {
                let mut f = flag.lock().unwrap();
                *f |= 1;
            }

            drop(gb);
            drop(ga);
        }));
    }

    // Worker b: takes b first (handshake), then releases b and re-acquires
    // in the global order a -> b to avoid lock-order inversion.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);

        handles.push(thread::spawn(move || {
            let gb = b.lock().unwrap();
            sb.release();
            sa.acquire();
            // Release b so we can acquire in the consistent order a -> b.
            drop(gb);
            let ga = a.lock().unwrap();
            let gb = b.lock().unwrap();

            {
                let mut f = flag.lock().unwrap();
                *f |= 2;
            }

            drop(gb);
            drop(ga);
        }));
    }

    // Bystander task: keeps making progress without ever finishing on its own.
    {
        let flag = Arc::clone(&flag);
        thread::spawn(move || loop {
            let mut f = flag.lock().unwrap();
            *f ^= 0;
            drop(f);
            thread::yield_now();
        });
    }

    for h in handles {
        h.join().unwrap();
    }

    let f = flag.lock().unwrap();
    let a_val = *f & 1;
    let b_val = (*f >> 1) & 1;
    println!("DONE a={} b={}", a_val, b_val);
}
