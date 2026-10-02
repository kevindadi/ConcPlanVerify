mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
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

    // Let a take b, then acquire both locks in a safe order.
    drop(b_guard);
    let a_guard = shared.a.lock().expect("lock a poisoned");
    let b_guard = shared.b.lock().expect("lock b poisoned");

    // Critical section: both locks are held.
    drop(b_guard);
    drop(a_guard);
    1
}

fn bystander(shared: Arc<Shared>) {
    loop {
        shared.flag.fetch_add(1, Ordering::Relaxed);
        thread::yield_now();
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        a: Mutex::new_named("a#1904", ()),
        b: Mutex::new_named("b#1931", ()),
        sa: Semaphore::new(),
        sb: Semaphore::new(),
        flag: AtomicUsize::new(0),
    });

    let a_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("a#2115", move || a(shared))
    };

    let b_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("b#2227", move || b(shared))
    };

    let bystander_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("bystander#2347", move || bystander(shared))
    };
    drop(bystander_handle);

    // Ensure the bystander has started; it then continues independently.
    while shared.flag.load(Ordering::Relaxed) == 0 {
        thread::yield_now();
    }

    let a_result = a_handle.join().expect("worker a panicked");
    let b_result = b_handle.join().expect("worker b panicked");

    println!("DONE a={} b={}", a_result, b_result);
 cir_trace::finish();}
