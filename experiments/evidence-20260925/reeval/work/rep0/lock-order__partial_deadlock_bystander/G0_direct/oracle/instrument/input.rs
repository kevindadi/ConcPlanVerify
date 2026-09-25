use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Counting semaphore implemented with Mutex + Condvar.
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            count: Mutex::new(initial),
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

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
    sa: Semaphore,
    sb: Semaphore,
    flag: Mutex<i32>,
}

fn main() {
    let shared = Arc::new(Shared {
        a: Mutex::new(()),
        b: Mutex::new(()),
        sa: Semaphore::new(0),
        sb: Semaphore::new(0),
        flag: Mutex::new(0),
    });

    let s_a = Arc::clone(&shared);
    let s_b = Arc::clone(&shared);
    let s_by = Arc::clone(&shared);

    // Worker a
    let ha = thread::spawn(move || {
        // Take first lock a
        let _ga = s_a.a.lock().unwrap();
        // Signal that a has taken its first lock
        s_a.sa.release();
        // Wait for b to take its first lock
        s_a.sb.acquire();
        // Now take second lock b
        let _gb = s_a.b.lock().unwrap();

        // Critical section: both locks held
        {
            let mut f = s_a.flag.lock().unwrap();
            *f += 1;
        }

        // Release locks (drop guards)
        drop(_gb);
        drop(_ga);
    });

    // Worker b
    let hb = thread::spawn(move || {
        // Take first lock b
        let _gb = s_b.b.lock().unwrap();
        // Signal that b has taken its first lock
        s_b.sb.release();
        // Wait for a to take its first lock
        s_b.sa.acquire();
        // Now take second lock a
        let _ga = s_b.a.lock().unwrap();

        // Critical section: both locks held
        {
            let mut f = s_b.flag.lock().unwrap();
            *f += 1;
        }

        // Release locks (drop guards)
        drop(_ga);
        drop(_gb);
    });

    // Bystander: keeps making progress, never finishes on its own.
    let hby = thread::spawn(move || {
        loop {
            // Do a small unit of independent work.
            let mut f = s_by.flag.lock().unwrap();
            *f += 0;
            drop(f);
            // Yield to avoid starving others; still makes progress.
            thread::yield_now();
        }
    });

    // Main waits only for the two workers.
    ha.join().unwrap();
    hb.join().unwrap();

    // Bystander is detached in spirit; we must not join it (it never ends).
    // But we need main to terminate. We can simply not join hby.
    // To avoid a leaked thread warning, we forget the handle.
    std::mem::forget(hby);

    let f = shared.flag.lock().unwrap();
    // flag should be 2 from the two workers (bystander adds 0).
    println!("DONE a=1 b=1");
    let _ = *f;
}
