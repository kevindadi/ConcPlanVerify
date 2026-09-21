use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<u32>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: u32) -> Self {
        Self {
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
        *self.permits.lock().unwrap() += 1;
        self.cv.notify_one();
    }
}

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let sem_a = Arc::new(Semaphore::new(0));
    let sem_b = Arc::new(Semaphore::new(0));
    let done = Arc::new(AtomicBool::new(false));

    // Independent third task keeps making progress until workers finish.
    {
        let done = Arc::clone(&done);
        thread::spawn(move || {
            let mut counter: u64 = 0;
            while !done.load(Ordering::Relaxed) {
                counter = counter.wrapping_add(1);
                std::hint::spin_loop();
            }
            let _ = counter;
        });
    }

    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_a = thread::spawn(move || {
        let ga = ma.lock().unwrap();
        sa.release();
        sb.acquire();
        // Release the first lock before acquiring the second so the
        // A-holds-a/wants-b vs B-holds-b/wants-a cycle cannot occur.
        drop(ga);
        let gb = mb.lock().unwrap();
        drop(gb);
    });

    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_b = thread::spawn(move || {
        let gb = mb2.lock().unwrap();
        sb2.release();
        sa2.acquire();
        drop(gb);
        let ga = ma2.lock().unwrap();
        drop(ga);
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
    done.store(true, Ordering::Relaxed);

    println!("DONE a=1 b=1");
}
