mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Counting semaphore implemented with Mutex + Condvar.
struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            count: Mutex::new(permits),
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

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
    sa: Semaphore,
    sb: Semaphore,
    flag: Mutex<(bool, bool)>,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        a: Mutex::new_named("shared_mutex0", ()),
        b: Mutex::new_named("shared_mutex1", ()),
        sa: Semaphore::new(0),
        sb: Semaphore::new(0),
        flag: Mutex::new_named("shared_mutex2", (false, false)),
    });

    let s_a = Arc::clone(&shared);
    let s_b = Arc::clone(&shared);
    let s_by = Arc::clone(&shared);

    // Worker a
    let ha = cir_trace::spawn("ha", move || {
        // Take first lock a.
        let _ga = s_a.a.lock().unwrap();
        // Signal that a has taken its first lock.
        s_a.sa.release();
        // Wait for b to take its first lock before taking second lock.
        s_a.sb.acquire();
        // Take second lock b.
        let _gb = s_a.b.lock().unwrap();
        // Critical section: both locks held.
        {
            let mut f = s_a.flag.lock().unwrap();
            f.0 = true;
        }
        // Release locks (drop guards) before finishing.
        drop(_gb);
        drop(_ga);
    });

    // Worker b
    let hb = cir_trace::spawn("hb", move || {
        // Take first lock b.
        let _gb = s_b.b.lock().unwrap();
        // Signal that b has taken its first lock.
        s_b.sb.release();
        // Wait for a to take its first lock before taking second lock.
        s_b.sa.acquire();
        // Take second lock a.
        let _ga = s_b.a.lock().unwrap();
        // Critical section: both locks held.
        {
            let mut f = s_b.flag.lock().unwrap();
            f.1 = true;
        }
        // Release locks (drop guards) before finishing.
        drop(_ga);
        drop(_gb);
    });

    // Bystander: keeps making progress, never finishes on its own.
    let hby = cir_trace::spawn("yield_now", move || {
        loop {
            // Do a small unit of independent work.
            let _ = s_by.flag.lock().unwrap();
            // Yield to avoid busy-spinning too hard, but keep running.
            thread::yield_now();
        }
    });

    // Main waits for both workers to finish.
    ha.join().unwrap();
    hb.join().unwrap();

    // Read final flag values.
    let (a_done, b_done) = {
        let f = shared.flag.lock().unwrap();
        (f.0, f.1)
    };

    println!("DONE a={} b={}", a_done as u32, b_done as u32);

    // Bystander never finishes; detach it by not joining.
    // Process exits when main returns.
    drop(hby);
 cir_trace::finish();}
