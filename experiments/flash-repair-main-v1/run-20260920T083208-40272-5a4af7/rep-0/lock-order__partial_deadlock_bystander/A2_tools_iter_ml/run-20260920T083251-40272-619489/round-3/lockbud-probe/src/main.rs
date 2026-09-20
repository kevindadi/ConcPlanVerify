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

    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_a = thread::spawn(move || {
        // Handshake: signal our readiness, then wait for the peer.
        // Both releases happen before any lock is taken, so no thread
        // holds a mutex while blocking on a semaphore.
        sa.release();
        sb.acquire();
        // Consistent lock order: mtx_a then mtx_b.
        let ga = ma.lock().unwrap();
        let gb = mb.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_b = thread::spawn(move || {
        sb2.release();
        sa2.acquire();
        // Same lock order: mtx_a then mtx_b.
        let ga = ma2.lock().unwrap();
        let gb = mb2.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    // Independent third task that keeps making progress and terminates
    // once both workers have completed.
    let done = Arc::new((Mutex::new(false), Condvar::new()));
    let done_clone = Arc::clone(&done);
    let bystander = thread::spawn(move || {
        let (lock, cv) = &*done_clone;
        let mut finished = lock.lock().unwrap();
        while !*finished {
            finished = cv.wait(finished).unwrap();
        }
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Signal the bystander to finish so it does not leak.
    {
        let (lock, cv) = &*done;
        let mut finished = lock.lock().unwrap();
        *finished = true;
        cv.notify_all();
    }
    bystander.join().unwrap();

    println!("DONE a=1 b=1");
}
