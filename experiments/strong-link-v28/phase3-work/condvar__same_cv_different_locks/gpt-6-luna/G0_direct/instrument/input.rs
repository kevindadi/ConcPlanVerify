use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            count: Mutex::new(0),
            cv: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Condvar>, ready: Arc<Semaphore>) {
    let mut guard = m1.lock().unwrap();
    ready.release();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Condvar>, ready: Arc<Semaphore>) {
    let mut guard = m2.lock().unwrap();
    ready.release();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(
    m1: Arc<Mutex<bool>>,
    m2: Arc<Mutex<bool>>,
    cv: [Arc<Condvar>; 2],
    ready: Arc<Semaphore>,
) {
    ready.acquire();
    ready.acquire();

    let mut guard1 = m1.lock().unwrap();
    let mut guard2 = m2.lock().unwrap();

    *guard1 = true;
    *guard2 = true;
    cv[0].notify_all();
    cv[1].notify_all();
}

fn main() {
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));

    // Each condition variable is paired with its waiter's own mutex.
    let cv = [Arc::new(Condvar::new()), Arc::new(Condvar::new())];
    let ready = Arc::new(Semaphore::new());

    let t1 = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv[0]);
        let ready = Arc::clone(&ready);
        thread::spawn(move || w1(m1, cv, ready))
    };

    let t2 = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv[1]);
        let ready = Arc::clone(&ready);
        thread::spawn(move || w2(m2, cv, ready))
    };

    let tn = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = [Arc::clone(&cv[0]), Arc::clone(&cv[1])];
        let ready = Arc::clone(&ready);
        thread::spawn(move || notifier(m1, m2, cv, ready))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    println!("DONE done=1");
}
