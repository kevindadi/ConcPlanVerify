use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Condvar, Mutex,
};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
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

fn waiter(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    remaining: Arc<AtomicUsize>,
) {
    let mut proceed = m.lock().unwrap();

    // Tell the notifier this waiter is ready, then wait while holding m.
    g12.release();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    remaining.fetch_sub(1, Ordering::SeqCst);
}

fn w1(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    remaining: Arc<AtomicUsize>,
) {
    waiter(m, cv, g12, remaining);
}

fn w2(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    remaining: Arc<AtomicUsize>,
) {
    waiter(m, cv, g12, remaining);
}

fn notifier(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
) {
    // Wait until both waiters have announced they are ready.
    g12.acquire();
    g12.acquire();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    gN.release();
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));
    let remaining = Arc::new(AtomicUsize::new(2));

    let w1_handle = {
        let (m, cv, g12, remaining) = (
            Arc::clone(&m),
            Arc::clone(&cv),
            Arc::clone(&g12),
            Arc::clone(&remaining),
        );
        thread::spawn(move || w1(m, cv, g12, remaining))
    };

    let w2_handle = {
        let (m, cv, g12, remaining) = (
            Arc::clone(&m),
            Arc::clone(&cv),
            Arc::clone(&g12),
            Arc::clone(&remaining),
        );
        thread::spawn(move || w2(m, cv, g12, remaining))
    };

    let notifier_handle = {
        let (m, cv, g12, gN) = (
            Arc::clone(&m),
            Arc::clone(&cv),
            Arc::clone(&g12),
            Arc::clone(&gN),
        );
        thread::spawn(move || notifier(m, cv, g12, gN))
    };

    gN.acquire();

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters={}", remaining.load(Ordering::SeqCst));
}
