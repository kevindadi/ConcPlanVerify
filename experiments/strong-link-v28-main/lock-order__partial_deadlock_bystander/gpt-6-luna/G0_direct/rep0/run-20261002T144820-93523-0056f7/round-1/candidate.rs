use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    ready: Condvar,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            count: Mutex::new(0),
            ready: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.ready.notify_one();
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.ready.wait(count).unwrap();
        }
        *count -= 1;
    }
}

struct Resources {
    a: Mutex<()>,
    b: Mutex<()>,
    sa: Semaphore,
    sb: Semaphore,
    flag: AtomicUsize,
}

fn a(resources: Arc<Resources>) -> usize {
    // Take the first lock and announce it before waiting for b's announcement.
    let first = resources.a.lock().unwrap();
    resources.flag.fetch_or(1, Ordering::SeqCst);
    resources.sa.release();
    resources.sb.acquire();
    drop(first);

    // Both workers now use the same lock order, avoiding a lock-order cycle.
    let first = resources.a.lock().unwrap();
    let second = resources.b.lock().unwrap();
    black_box(());
    drop(second);
    drop(first);

    1
}

fn b(resources: Arc<Resources>) -> usize {
    // Take the first lock and announce it before waiting for a's announcement.
    let first = resources.b.lock().unwrap();
    resources.flag.fetch_or(2, Ordering::SeqCst);
    resources.sb.release();
    resources.sa.acquire();
    drop(first);

    // Both workers now use the same lock order, avoiding a lock-order cycle.
    let first = resources.a.lock().unwrap();
    let second = resources.b.lock().unwrap();
    black_box(());
    drop(second);
    drop(first);

    1
}

fn bystander() {
    let mut progress = 0usize;
    loop {
        progress = progress.wrapping_add(1);
        black_box(progress);
    }
}

fn main() {
    let resources = Arc::new(Resources {
        a: Mutex::new(()),
        b: Mutex::new(()),
        sa: Semaphore::new(),
        sb: Semaphore::new(),
        flag: AtomicUsize::new(0),
    });

    let a_thread = {
        let resources = Arc::clone(&resources);
        thread::spawn(move || a(resources))
    };
    let b_thread = {
        let resources = Arc::clone(&resources);
        thread::spawn(move || b(resources))
    };
    drop(thread::spawn(bystander));

    let a_result = a_thread.join().unwrap();
    let b_result = b_thread.join().unwrap();

    println!("DONE a={} b={}", a_result, b_result);
}
