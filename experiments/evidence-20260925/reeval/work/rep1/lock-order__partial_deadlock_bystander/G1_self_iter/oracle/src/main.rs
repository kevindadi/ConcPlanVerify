mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use std::time::Duration;

// Counting semaphore implemented with Mutex + Condvar
struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
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

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0", 0));
    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));
    let flag = Arc::new(Mutex::new_named("flag_mutex0", 0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let sa1 = Arc::clone(&sa);
    let sb1 = Arc::clone(&sb);
    let flag1 = Arc::clone(&flag);

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let sa2 = Arc::clone(&sa);
    let sb2 = Arc::clone(&sb);
    let flag2 = Arc::clone(&flag);

    // Worker a
    let worker_a = cir_trace::spawn("worker_a", move || {
        // Take first lock a
        let _ga = a1.lock().unwrap();
        // Signal that a has taken its first lock
        sa1.release();
        // Wait for b to take its first lock
        sb1.acquire();
        // Take second lock b
        let _gb = b1.lock().unwrap();
        // Critical section
        {
            let mut f = flag1.lock().unwrap();
            *f += 1;
        }
        // Locks released automatically at end of scope
    });

    // Worker b
    let worker_b = cir_trace::spawn("worker_b", move || {
        // Take first lock b
        let _gb = b2.lock().unwrap();
        // Signal that b has taken its first lock
        sb2.release();
        // Wait for a to take its first lock
        sa2.acquire();
        // Take second lock a
        let _ga = a2.lock().unwrap();
        // Critical section
        {
            let mut f = flag2.lock().unwrap();
            *f += 1;
        }
        // Locks released automatically at end of scope
    });

    // Bystander task: keeps making progress, never finishes on its own
    let bystander = cir_trace::spawn("sleep", move || {
        loop {
            thread::sleep(Duration::from_millis(10));
        }
    });

    // Main thread waits for both workers to finish
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Bystander keeps running; we don't join it.
    // Print the required line and exit.
    println!("DONE a=1 b=1");

    // Detach bystander by dropping the handle; process exits.
    drop(bystander);
 cir_trace::finish();}
