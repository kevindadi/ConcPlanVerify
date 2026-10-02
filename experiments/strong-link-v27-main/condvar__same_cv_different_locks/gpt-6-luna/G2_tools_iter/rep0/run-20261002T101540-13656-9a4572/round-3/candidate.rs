use std::sync::{Arc, Condvar, Mutex};
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

// Each waiter uses a condition variable paired with its own data lock.
struct Cv {
    w1: Condvar,
    w2: Condvar,
}

impl Cv {
    fn new() -> Self {
        Self {
            w1: Condvar::new(),
            w2: Condvar::new(),
        }
    }
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut data = m1.lock().unwrap();
    ready.release();

    while !*data {
        data = cv.w1.wait(data).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut data = m2.lock().unwrap();
    ready.release();

    while !*data {
        data = cv.w2.wait(data).unwrap();
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

    // Use a consistent lock order, and hold both locks while waking the waiters.
    let mut data1 = m1.lock().unwrap();
    let mut data2 = m2.lock().unwrap();

    *data1 = true;
    *data2 = true;
    cv.w1.notify_one();
    cv.w2.notify_one();
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
        thread::spawn(move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        thread::spawn(move || w2(m2, cv, ready))
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        thread::spawn(move || notifier(m1, m2, cv, ready))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
}
