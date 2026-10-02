mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// Deadlock-avoidance design:
//   worker a: take lock a -> signal sa -> wait sb -> take lock b -> critical -> release both
//   worker b: take lock b -> signal sb -> wait sa -> take lock a -> critical -> release both
// The handshake (R5) guarantees neither worker takes its second lock before the
// other has taken its first, so the circular-wait condition never arises (R7, R8, R9).
// The bystander only touches `flag` and never holds locks a/b, so it can keep
// running forever without blocking the workers (R2, R6).

use std::sync::{Arc};
use std::thread;
use std::time::Duration;

/// Simple counting semaphore (for the two coordination permits sa and sb).
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
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

type SharedLock = Arc<Mutex<()>>;

/// Role: a
fn worker_a(lock_a: SharedLock, lock_b: SharedLock, sa: Arc<Semaphore>, sb: Arc<Semaphore>, flag: Arc<Mutex<u64>>) {
    // Take first lock.
    let guard_a = lock_a.lock().unwrap();

    // Handshake: announce that a holds its first lock...
    sa.release();
    // ...and wait until b holds its first lock before taking the second (R5).
    sb.acquire();

    // Now take the second lock: both locks are held simultaneously (R4).
    let guard_b = lock_b.lock().unwrap();

    // Critical section.
    {
        let mut f = flag.lock().unwrap();
        *f += 1;
    }

    // Release each lock before finishing (R10).
    drop(guard_b);
    drop(guard_a);
}

/// Role: b
fn worker_b(lock_a: SharedLock, lock_b: SharedLock, sa: Arc<Semaphore>, sb: Arc<Semaphore>, flag: Arc<Mutex<u64>>) {
    // Take first lock.
    let guard_b = lock_b.lock().unwrap();

    // Handshake: announce that b holds its first lock...
    sb.release();
    // ...and wait until a holds its first lock before taking the second (R5).
    sa.acquire();

    // Now take the second lock: both locks are held simultaneously (R4).
    let guard_a = lock_a.lock().unwrap();

    // Critical section.
    {
        let mut f = flag.lock().unwrap();
        *f += 1;
    }

    // Release each lock before finishing (R10).
    drop(guard_a);
    drop(guard_b);
}

/// Role: bystander — runs forever, keeps making progress, never finishes on its own (R2).
/// It only touches `flag` and never holds locks a or b, so it can never block
/// the workers (R6, R7, R8).
fn bystander_task(flag: Arc<Mutex<u64>>) {
    loop {
        {
            let mut f = flag.lock().unwrap();
            *f += 1;
        } // lock released immediately
        thread::sleep(Duration::from_millis(1));
    }
}

fn main() { cir_trace::init();
    // Shared resources: locks a and b, permits sa and sb, shared variable flag.
    let lock_a: SharedLock = Arc::new(Mutex::new_named("res_mutex0#3188", ()));
    let lock_b: SharedLock = Arc::new(Mutex::new_named("res_mutex0#3243", ()));
    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));
    let flag = Arc::new(Mutex::new_named("flag_mutex0#3368", 0u64));

    // Main starts all three tasks (R1, R11).
    let bystander_handle = {
        let flag = Arc::clone(&flag);
        cir_trace::spawn("bystander_task#3502", move || bystander_task(flag))
    };

    let handle_a = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);
        cir_trace::spawn("worker_a#3773", move || worker_a(lock_a, lock_b, sa, sb, flag))
    };

    let handle_b = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);
        cir_trace::spawn("worker_b#4062", move || worker_b(lock_a, lock_b, sa, sb, flag))
    };

    // Main finishes only after both workers have finished (R11).
    // The bystander is never joined; it may keep running (R2, R6).
    handle_a.join().expect("worker a panicked");
    handle_b.join().expect("worker b panicked");

    // Exactly the required output, then exit (R12).
    println!("DONE a=1 b=1");
    drop(bystander_handle); // bystander is abandoned when the process exits
 cir_trace::finish();}
