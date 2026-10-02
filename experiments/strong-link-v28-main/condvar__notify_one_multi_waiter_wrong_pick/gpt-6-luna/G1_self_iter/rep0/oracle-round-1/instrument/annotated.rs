mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            permits: Mutex::new(0),
            cv: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cv.notify_one();
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }
}

struct Shared {
    m: Mutex<bool>,
    cv: Condvar,
    g12: Semaphore,
    gN: Semaphore,
    remaining: AtomicUsize,
}

fn waiter(shared: Arc<Shared>) {
    let mut proceed = shared.m.lock().unwrap();

    // Announce readiness while holding m. The notifier must acquire m
    // after receiving both permits, so each waiter will have entered wait.
    shared.g12.release();

    while !*proceed {
        proceed = shared.cv.wait(proceed).unwrap();
    }
    drop(proceed);

    shared.remaining.fetch_sub(1, Ordering::SeqCst);
    shared.gN.release();
}

fn w1(shared: Arc<Shared>) {
    waiter(shared);
}

fn w2(shared: Arc<Shared>) {
    waiter(shared);
}

fn notifier(shared: Arc<Shared>) {
    shared.g12.acquire();
    shared.g12.acquire();

    let mut proceed = shared.m.lock().unwrap();
    *proceed = true;
    shared.cv.notify_all();
    drop(proceed);

    // Wait for both waiters to finish.
    shared.gN.acquire();
    shared.gN.acquire();
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("m#1679", false),
        cv: Condvar::new_named("cv#1712"),
        g12: Semaphore::new(),
        gN: Semaphore::new(),
        remaining: AtomicUsize::new(2),
    });

    let h1 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w1#1894", move || w1(shared))
    };
    let h2 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w2#2000", move || w2(shared))
    };
    let hn = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#2106", move || notifier(shared))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    hn.join().unwrap();

    println!(
        "DONE waiters={}",
        shared.remaining.load(Ordering::SeqCst)
    );
 cir_trace::finish();}
