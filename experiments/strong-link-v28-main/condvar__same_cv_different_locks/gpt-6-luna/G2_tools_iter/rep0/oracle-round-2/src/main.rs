mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Self {
            count: Mutex::new(count),
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

// Each waiter uses its own condition variable with its own lock.
struct Cv {
    for_w1: Condvar,
    for_w2: Condvar,
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut notified = m1.lock().unwrap();
    ready.release();

    while !*notified {
        notified = cv.for_w1.wait(notified).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut notified = m2.lock().unwrap();
    ready.release();

    while !*notified {
        notified = cv.for_w2.wait(notified).unwrap();
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

    // Wake each waiter while holding its lock, without holding both locks
    // at the same time. The predicates also prevent missed notifications.
    {
        let mut notified = m1.lock().unwrap();
        *notified = true;
        cv.for_w1.notify_all();
    }

    {
        let mut notified = m2.lock().unwrap();
        *notified = true;
        cv.for_w2.notify_all();
    }
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0#1772", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0#1814", false));
    let cv = Arc::new(Cv {
        for_w1: Condvar::new_named("for_w1#1879"),
        for_w2: Condvar::new_named("for_w2#1911"),
    });
    let ready = Arc::new(Semaphore::new(0));

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w1#2110", move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w2#2297", move || w2(m2, cv, ready))
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("notifier#2524", move || notifier(m1, m2, cv, ready))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
