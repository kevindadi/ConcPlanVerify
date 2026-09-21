use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}};
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

    // Stop flag and progress counter for the independent third task.
    let stop = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicU64::new(0));

    // Worker A: lock mtx_a, semaphore handshake, then lock mtx_b.
    // The first guard is dropped before acquiring the second mutex,
    // which removes the circular wait (A holds mtx_a wanting mtx_b
    // while B holds mtx_b wanting mtx_a) that deadlocked the original.
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
        drop(ga);
        let gb = mb.lock().unwrap();
        drop(gb);
    });

    // Worker B: lock mtx_b, semaphore handshake, then lock mtx_a.
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

    // Independent third task: keeps making progress (increments a counter)
    // until asked to stop. Coordination is via the atomic flag, no sleeping.
    let (stop3, progress3) = (Arc::clone(&stop), Arc::clone(&progress));
    let third = thread::spawn(move || {
        while !stop3.load(Ordering::Acquire) {
            progress3.fetch_add(1, Ordering::Relaxed);
        }
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Stop and join the third task so every spawned thread is joined.
    stop.store(true, Ordering::Release);
    third.join().unwrap();

    println!("DONE a=1 b=1");
}
