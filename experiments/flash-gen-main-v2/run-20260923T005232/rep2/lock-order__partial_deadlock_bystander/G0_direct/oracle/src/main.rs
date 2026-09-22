mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Counting semaphore implemented with Mutex + Condvar.
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

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
    sa: Semaphore,
    sb: Semaphore,
    flag: Mutex<i32>,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        a: Mutex::new_named("shared_mutex0", ()),
        b: Mutex::new_named("shared_mutex1", ()),
        sa: Semaphore::new(0),
        sb: Semaphore::new(0),
        flag: Mutex::new_named("shared_mutex2", 0),
    });

    let s_a = Arc::clone(&shared);
    let s_b = Arc::clone(&shared);
    let s_by = Arc::clone(&shared);

    // Worker a
    let ha = cir_trace::spawn("ha", move || {
        // Take first lock a
        let _ga = s_a.a.lock().unwrap();
        // Signal that a has taken its first lock
        s_a.sa.release();
        // Wait for b to take its first lock before taking second lock
        s_a.sb.acquire();
        // Take second lock b
        let _gb = s_a.b.lock().unwrap();
        // Critical section: both locks held
        {
            let mut f = s_a.flag.lock().unwrap();
            *f += 1;
        }
        // Release locks (drop guards) before finishing
        drop(_gb);
        drop(_ga);
    });

    // Worker b
    let hb = cir_trace::spawn("hb", move || {
        // Take first lock b
        let _gb = s_b.b.lock().unwrap();
        // Signal that b has taken its first lock
        s_b.sb.release();
        // Wait for a to take its first lock before taking second lock
        s_b.sa.acquire();
        // Take second lock a
        let _ga = s_b.a.lock().unwrap();
        // Critical section: both locks held
        {
            let mut f = s_b.flag.lock().unwrap();
            *f += 1;
        }
        // Release locks (drop guards) before finishing
        drop(_ga);
        drop(_gb);
    });

    // Bystander: keeps making progress without ever finishing on its own.
    // It must not prevent workers from finishing, so it never holds a or b
    // for long and never blocks on the permits.
    let hby = cir_trace::spawn("hby", move || {
        let mut counter: u64 = 0;
        loop {
            // Do some independent work that does not hold the workers' locks.
            counter = counter.wrapping_add(1);
            // Occasionally touch the shared flag under its own lock briefly,
            // but never hold a or b.
            if counter % 1_000_000 == 0 {
                let mut f = s_by.flag.lock().unwrap();
                *f += 0;
            }
        }
    });

    // Main waits only for the two workers.
    ha.join().unwrap();
    hb.join().unwrap();

    // Bystander never finishes on its own; detach it by not joining.
    // (We still keep the handle to avoid unused warning.)
    drop(hby);

    let f = shared.flag.lock().unwrap();
    println!("DONE a=1 b=1");
    let _ = *f;
 cir_trace::finish();}
