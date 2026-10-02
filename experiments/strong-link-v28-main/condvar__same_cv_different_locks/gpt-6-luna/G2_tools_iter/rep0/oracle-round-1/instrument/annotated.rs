mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, MutexGuard};
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

// One logical condition variable, with one underlying Condvar per lock.
// cir_trace::sync::Condvar cannot safely be used with multiple Mutex values.
struct Cv {
    for_w1: Condvar,
    for_w2: Condvar,
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut notified: MutexGuard<'_, bool> = m1.lock().unwrap();
    ready.release(); // Announce while holding m1.
    while !*notified {
        notified = cv.for_w1.wait(notified).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut notified: MutexGuard<'_, bool> = m2.lock().unwrap();
    ready.release(); // Announce while holding m2.
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
    // Wait for both waiters to announce before attempting to wake them.
    ready.acquire();
    ready.acquire();

    // Each waiter needs its own lock to wake and finish.
    let mut notified1 = m1.lock().unwrap();
    let mut notified2 = m2.lock().unwrap();

    *notified1 = true;
    *notified2 = true;
    cv.for_w1.notify_all();
    cv.for_w2.notify_all();

    // The waiters proceed once these guards are released.
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0#1962", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0#2004", false));
    let cv = Arc::new(Cv {
        for_w1: Condvar::new_named("for_w1#2069"),
        for_w2: Condvar::new_named("for_w2#2101"),
    });
    let ready = Arc::new(Semaphore::new(0));

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w1#2300", move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w2#2487", move || w2(m2, cv, ready))
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("notifier#2714", move || notifier(m1, m2, cv, ready))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
