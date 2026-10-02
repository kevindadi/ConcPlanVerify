use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Condvar, Mutex,
};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    ready: Condvar,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            permits: Mutex::new(0),
            ready: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().expect("semaphore mutex poisoned");
        *permits += 1;
        self.ready.notify_one();
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().expect("semaphore mutex poisoned");
        while *permits == 0 {
            permits = self.ready.wait(permits).expect("semaphore mutex poisoned");
        }
        *permits -= 1;
    }
}

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
    sa: Semaphore,
    sb: Semaphore,
    flag: AtomicUsize,
}

fn a(shared: Arc<Shared>) -> usize {
    let a_guard = shared.a.lock().expect("lock a poisoned");
    shared.sa.release();

    // Wait until b has taken its first lock.
    shared.sb.acquire();
    let b_guard = shared.b.lock().expect("lock b poisoned");

    // Critical section: both locks are held.
    drop(b_guard);
    drop(a_guard);
    1
}

fn b(shared: Arc<Shared>) -> usize {
    let b_guard = shared.b.lock().expect("lock b poisoned");
    shared.sb.release();

    // Wait until a has taken its first lock.
    shared.sa.acquire();

    // Release b so a can take its second lock; then acquire both locks.
    drop(b_guard);
    let a_guard = shared.a.lock().expect("lock a poisoned");
    let b_guard = shared.b.lock().expect("lock b poisoned");

    // Critical section: both locks are held.
    drop(b_guard);
    drop(a_guard);
    1
}

fn bystander(shared: Arc<Shared>) {
    const STOP: usize = 1usize << (usize::BITS - 1);

    loop {
        let current = shared.flag.load(Ordering::Relaxed);
        if current & STOP != 0 {
            break;
        }

        let next = current.wrapping_add(1) & !STOP;
        if shared
            .flag
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            thread::yield_now();
        }
    }
}

fn main() {
    const STOP: usize = 1usize << (usize::BITS - 1);

    let shared = Arc::new(Shared {
        a: Mutex::new(()),
        b: Mutex::new(()),
        sa: Semaphore::new(),
        sb: Semaphore::new(),
        flag: AtomicUsize::new(0),
    });

    let a_handle = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || a(shared))
    };

    let b_handle = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || b(shared))
    };

    let bystander_handle = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || bystander(shared))
    };

    // Ensure the bystander has started and made progress.
    while shared.flag.load(Ordering::Relaxed) == 0 {
        thread::yield_now();
    }

    let a_result = a_handle.join().expect("worker a panicked");
    let b_result = b_handle.join().expect("worker b panicked");

    // Stop and join the bystander after both workers have finished.
    shared.flag.fetch_or(STOP, Ordering::Relaxed);
    bystander_handle.join().expect("bystander panicked");

    println!("DONE a={} b={}", a_result, b_result);
}
