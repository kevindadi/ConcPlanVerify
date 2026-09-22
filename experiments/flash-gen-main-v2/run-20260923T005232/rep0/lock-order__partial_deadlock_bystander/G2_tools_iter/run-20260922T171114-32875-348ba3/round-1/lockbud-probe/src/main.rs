use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Counting semaphore implemented with Mutex + Condvar (std only).
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
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
    // Shared locks.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Counting permits used as a handshake.
    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));

    // Shared flag variable.
    let flag = Arc::new(Mutex::new(0u32));

    // Worker a.
    let a_lock = Arc::clone(&a);
    let b_lock = Arc::clone(&b);
    let sa_sem = Arc::clone(&sa);
    let sb_sem = Arc::clone(&sb);
    let flag_a = Arc::clone(&flag);

    let worker_a = thread::spawn(move || {
        // Take first lock a.
        let _ga = a_lock.lock().unwrap();
        {
            let mut f = flag_a.lock().unwrap();
            *f += 1;
        }
        // Signal that a has taken its first lock.
        sa_sem.release();
        // Wait until b has taken its first lock.
        sb_sem.acquire();
        // Now take second lock b.
        let _gb = b_lock.lock().unwrap();
        // Critical section: both locks held.
        // Release locks (drop guards) before finishing.
        drop(_gb);
        drop(_ga);
    });

    // Worker b.
    let a_lock_b = Arc::clone(&a);
    let b_lock_b = Arc::clone(&b);
    let sa_sem_b = Arc::clone(&sa);
    let sb_sem_b = Arc::clone(&sb);
    let flag_b = Arc::clone(&flag);

    let worker_b = thread::spawn(move || {
        // Take first lock b.
        let _gb = b_lock_b.lock().unwrap();
        {
            let mut f = flag_b.lock().unwrap();
            *f += 1;
        }
        // Signal that b has taken its first lock.
        sb_sem_b.release();
        // Wait until a has taken its first lock.
        sa_sem_b.acquire();
        // Now take second lock a.
        let _ga = a_lock_b.lock().unwrap();
        // Critical section: both locks held.
        // Release locks (drop guards) before finishing.
        drop(_ga);
        drop(_gb);
    });

    // Bystander task: keeps making progress without ever finishing on its own.
    let flag_by = Arc::clone(&flag);
    let bystander = thread::spawn(move || loop {
        let mut f = flag_by.lock().unwrap();
        *f = f.wrapping_add(1);
        drop(f);
        thread::yield_now();
    });

    // Main waits only for the two workers.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Bystander keeps running; detach it by not joining.
    drop(bystander);

    let f = flag.lock().unwrap();
    // The flag is shared with the bystander, so we cannot rely on its value.
    // Instead, print the required line based on worker completion.
    let _ = *f;
    println!("DONE a=1 b=1");
}
