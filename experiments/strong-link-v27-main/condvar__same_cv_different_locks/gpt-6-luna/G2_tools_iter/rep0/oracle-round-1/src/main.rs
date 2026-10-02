mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, MutexGuard};
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

// Each condition variable is paired with its waiter's own lock.
struct Cv {
    for_m1: Condvar,
    for_m2: Condvar,
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut notified = m1.lock().unwrap();
    ready.release(); // Announce that w1 is about to wait.

    while !*notified {
        notified = cv.for_m1.wait(notified).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut notified = m2.lock().unwrap();
    ready.release(); // Announce that w2 is about to wait.

    while !*notified {
        notified = cv.for_m2.wait(notified).unwrap();
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

    let mut notified1 = m1.lock().unwrap();
    let mut notified2 = m2.lock().unwrap();

    *notified1 = true;
    *notified2 = true;
    cv.for_m1.notify_all();
    cv.for_m2.notify_all();
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0#1649", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0#1691", false));
    let cv = Arc::new(Cv {
        for_m1: Condvar::new_named("for_m1#1756"),
        for_m2: Condvar::new_named("for_m2#1788"),
    });
    let ready = Arc::new(Semaphore::new());

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w1#1986", move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w2#2173", move || w2(m2, cv, ready))
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("notifier#2400", move || notifier(m1, m2, cv, ready))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
