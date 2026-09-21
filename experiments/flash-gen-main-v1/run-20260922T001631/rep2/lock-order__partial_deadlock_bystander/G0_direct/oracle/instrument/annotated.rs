mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// A counting semaphore implemented with Mutex + Condvar.
struct Semaphore {
    permits: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut p = self.permits.lock().unwrap();
        while *p == 0 {
            p = self.cond.wait(p).unwrap();
        }
        *p -= 1;
    }

    fn release(&self) {
        let mut p = self.permits.lock().unwrap();
        *p += 1;
        self.cond.notify_one();
    }
}

// Shared state: two locks and two permits.
struct Shared {
    lock_a: Mutex<()>,
    lock_b: Mutex<()>,
    // permit1: worker1 signals it has taken lock_a; worker2 waits before taking lock_b.
    permit1: Semaphore,
    // permit2: worker2 signals it has taken lock_a; worker1 waits before taking lock_b.
    permit2: Semaphore,
}

fn worker1(shared: Arc<Shared>, done: Arc<(Mutex<usize>, Condvar)>) {
    // Take first lock.
    let _ga = shared.lock_a.lock().unwrap();
    // Signal that we have taken our first lock.
    shared.permit1.release();
    // Wait until the other worker has taken its first lock.
    shared.permit2.acquire();
    // Now take second lock.
    let _gb = shared.lock_b.lock().unwrap();

    // Critical section: hold both locks.
    // (do work)

    // Release both locks (drop guards).
    drop(_gb);
    drop(_ga);

    // Mark done.
    let (m, c) = &*done;
    let mut count = m.lock().unwrap();
    *count += 1;
    c.notify_all();
}

fn worker2(shared: Arc<Shared>, done: Arc<(Mutex<usize>, Condvar)>) {
    // Take first lock.
    let _ga = shared.lock_a.lock().unwrap();
    // Signal that we have taken our first lock.
    shared.permit2.release();
    // Wait until the other worker has taken its first lock.
    shared.permit1.acquire();
    // Now take second lock.
    let _gb = shared.lock_b.lock().unwrap();

    // Critical section: hold both locks.
    // (do work)

    // Release both locks (drop guards).
    drop(_gb);
    drop(_ga);

    // Mark done.
    let (m, c) = &*done;
    let mut count = m.lock().unwrap();
    *count += 1;
    c.notify_all();
}

fn bystander(shared: Arc<Shared>) {
    // The bystander keeps making progress without ever finishing on its own.
    // It must not prevent the workers from finishing, so it never holds
    // both locks at once and never blocks on the permits.
    loop {
        // Briefly take and release lock_a.
        {
            let _g = shared.lock_a.lock().unwrap();
        }
        // Briefly take and release lock_b.
        {
            let _g = shared.lock_b.lock().unwrap();
        }
        // No sleep/yield; just loop. To avoid a tight spin that could starve
        // workers, we do a small amount of work between lock acquisitions.
        // (Still no timing primitives.)
        let mut x: u64 = 0;
        for i in 0..1000u64 {
            x = x.wrapping_add(i);
        }
        std::hint::black_box(x);
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        lock_a: Mutex::new_named("shared_mutex0", ()),
        lock_b: Mutex::new_named("shared_mutex1", ()),
        permit1: Semaphore::new(0),
        permit2: Semaphore::new(0),
    });

    let done = Arc::new((Mutex::new_named("done_mutex0", 0usize), Condvar::new_named("done_condvar0")));

    let s1 = Arc::clone(&shared);
    let d1 = Arc::clone(&done);
    let h1 = cir_trace::spawn("h1", move || worker1(s1, d1));

    let s2 = Arc::clone(&shared);
    let d2 = Arc::clone(&done);
    let h2 = cir_trace::spawn("h2", move || worker2(s2, d2));

    let sb = Arc::clone(&shared);
    let hb = cir_trace::spawn("hb", move || bystander(sb));

    // Wait for both workers to finish.
    {
        let (m, c) = &*done;
        let mut count = m.lock().unwrap();
        while *count < 2 {
            count = c.wait(count).unwrap();
        }
    }

    h1.join().unwrap();
    h2.join().unwrap();

    // The bystander never finishes on its own; we detach it by not joining.
    // But we must ensure main terminates. Dropping the handle detaches the thread.
    drop(hb);

    println!("DONE a=1 b=1");
 cir_trace::finish();}
