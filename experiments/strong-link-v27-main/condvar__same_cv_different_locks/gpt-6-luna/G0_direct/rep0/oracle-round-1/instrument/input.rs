use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            count: Mutex::new(0),
            available: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.available.notify_one();
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
    }
}

// One logical condition variable with a wait queue for each lock.
struct Cv {
    for_m1: Condvar,
    for_m2: Condvar,
}

impl Cv {
    fn new() -> Self {
        Self {
            for_m1: Condvar::new(),
            for_m2: Condvar::new(),
        }
    }

    fn wait_m1<'a>(&self, guard: MutexGuard<'a, bool>) -> MutexGuard<'a, bool> {
        self.for_m1.wait(guard).unwrap()
    }

    fn wait_m2<'a>(&self, guard: MutexGuard<'a, bool>) -> MutexGuard<'a, bool> {
        self.for_m2.wait(guard).unwrap()
    }

    fn notify_all(&self) {
        self.for_m1.notify_all();
        self.for_m2.notify_all();
    }
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut guard = m1.lock().unwrap();
    ready.release();

    while !*guard {
        guard = cv.wait_m1(guard);
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut guard = m2.lock().unwrap();
    ready.release();

    while !*guard {
        guard = cv.wait_m2(guard);
    }
}

fn notifier(
    m1: Arc<Mutex<bool>>,
    m2: Arc<Mutex<bool>>,
    cv: Arc<Cv>,
    ready: Arc<Semaphore>,
) {
    ready.acquire();
    ready.acquire();

    let mut guard1 = m1.lock().unwrap();
    let mut guard2 = m2.lock().unwrap();

    *guard1 = true;
    *guard2 = true;
    cv.notify_all();
}

fn main() {
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));
    let cv = Arc::new(Cv::new());
    let ready = Arc::new(Semaphore::new());

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        thread::Builder::new()
            .name("w1".into())
            .spawn(move || w1(m1, cv, ready))
            .unwrap()
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        thread::Builder::new()
            .name("w2".into())
            .spawn(move || w2(m2, cv, ready))
            .unwrap()
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        thread::Builder::new()
            .name("notifier".into())
            .spawn(move || notifier(m1, m2, cv, ready))
            .unwrap()
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
}
