mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            permits: Mutex::new(0),
            available: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
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
    let first_a = shared.a.lock().unwrap();
    shared.sa.release();
    shared.sb.acquire();

    let second_b = shared.b.lock().unwrap();
    shared.flag.fetch_add(1, Ordering::SeqCst);

    drop(second_b);
    drop(first_a);
    1
}

fn b(shared: Arc<Shared>) -> usize {
    let first_b = shared.b.lock().unwrap();
    shared.sb.release();
    shared.sa.acquire();

    // Release b before acquiring the locks in the common a-then-b order.
    drop(first_b);
    let first_a = shared.a.lock().unwrap();
    let second_b = shared.b.lock().unwrap();
    shared.flag.fetch_add(1, Ordering::SeqCst);

    drop(second_b);
    drop(first_a);
    1
}

fn bystander(progress: Arc<AtomicUsize>) {
    loop {
        { let __cpv = progress.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("progress#1851", (progress.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
        thread::yield_now();
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        a: Mutex::new_named("a#1692", ()),
        b: Mutex::new_named("b#1719", ()),
        sa: Semaphore::new(),
        sb: Semaphore::new(),
        flag: AtomicUsize::new(0),
    });

    let progress = Arc::new(AtomicUsize::new(0));
    let bystander_progress = Arc::clone(&progress);
    let _bystander_handle = cir_trace::spawn("bystander#1962", move || bystander(bystander_progress));

    // Ensure the bystander has started making progress before starting the workers.
    while { let __cpv = progress.load(Ordering::Relaxed); cir_trace::record_value("progress#1851", (__cpv) as i64); __cpv } == 0 {
        thread::yield_now();
    }

    let shared_a = Arc::clone(&shared);
    let worker_a = cir_trace::spawn("a#2247", move || a(shared_a));

    let shared_b = Arc::clone(&shared);
    let worker_b = cir_trace::spawn("b#2343", move || b(shared_b));

    let result_a = worker_a.join().unwrap();
    let result_b = worker_b.join().unwrap();

    println!("DONE a={result_a} b={result_b}");
 cir_trace::finish();}
