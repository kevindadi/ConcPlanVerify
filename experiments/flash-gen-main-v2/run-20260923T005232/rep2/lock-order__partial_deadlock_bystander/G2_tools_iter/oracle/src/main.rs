mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    // Shared locks.
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    // Counting permits used as a handshake.
    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));

    // Shared flag variable.
    let flag = Arc::new(Mutex::new_named("flag_mutex0", 0u32));

    // Worker a.
    let a_lock = Arc::clone(&a);
    let b_lock = Arc::clone(&b);
    let sa_a = Arc::clone(&sa);
    let sb_a = Arc::clone(&sb);
    let flag_a = Arc::clone(&flag);

    let worker_a = cir_trace::spawn("worker_a", move || {
        // Take first lock.
        let _ga = a_lock.lock().unwrap();

        // Signal that a has taken its first lock.
        sa_a.release();

        // Wait until b has taken its first lock before taking second lock.
        sb_a.acquire();

        // Take second lock while still holding the first.
        let _gb = b_lock.lock().unwrap();

        // Critical section.
        {
            let mut f = flag_a.lock().unwrap();
            *f += 1;
        }

        // Locks released automatically when guards drop.
    });

    // Worker b.
    let a_lock = Arc::clone(&a);
    let b_lock = Arc::clone(&b);
    let sa_b = Arc::clone(&sa);
    let sb_b = Arc::clone(&sb);
    let flag_b = Arc::clone(&flag);

    let worker_b = cir_trace::spawn("worker_b", move || {
        // Take first lock.
        let _gb = b_lock.lock().unwrap();

        // Signal that b has taken its first lock.
        sb_b.release();

        // Wait until a has taken its first lock before taking second lock.
        sa_b.acquire();

        // Take second lock while still holding the first.
        let _ga = a_lock.lock().unwrap();

        // Critical section.
        {
            let mut f = flag_b.lock().unwrap();
            *f += 1;
        }

        // Locks released automatically when guards drop.
    });

    // Bystander task: keeps making progress without ever finishing on its own.
    // It only briefly touches the shared flag and yields, so it never blocks
    // the workers from acquiring the flag lock.
    let flag_by = Arc::clone(&flag);
    let bystander = cir_trace::spawn("bystander", move || loop {
        {
            let mut f = flag_by.lock().unwrap();
            // Make progress: read and write the shared variable.
            *f = *f;
        }
        // Yield to allow other threads to run.
        thread::yield_now();
    });

    // Main waits only for the two workers.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Read final flag value.
    let final_flag = {
        let f = flag.lock().unwrap();
        *f
    };

    // The bystander may still be running; detach it by not joining.
    drop(bystander);

    // Print exactly the required line.
    println!("DONE a=1 b=1");

    // Ensure final_flag is used so the compiler doesn't warn.
    let _ = final_flag;
 cir_trace::finish();}
