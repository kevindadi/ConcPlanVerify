mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

struct WaiterData {
    notified: bool,
}

// Each condition variable in this shared `cv` is paired with the waiter's own
// lock, as required by Rust's Condvar API.
struct Cv {
    for_w1: Condvar,
    for_w2: Condvar,
}

fn w1(m1: Arc<Mutex<WaiterData>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut data = m1.lock().unwrap();

    // Announce readiness before blocking.
    ready.release();

    while !data.notified {
        data = cv.for_w1.wait(data).unwrap();
    }
}

fn w2(m2: Arc<Mutex<WaiterData>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let mut data = m2.lock().unwrap();

    // Announce readiness before blocking.
    ready.release();

    while !data.notified {
        data = cv.for_w2.wait(data).unwrap();
    }
}

fn notifier(
    m1: Arc<Mutex<WaiterData>>,
    m2: Arc<Mutex<WaiterData>>,
    cv: Arc<Cv>,
    ready: Arc<Semaphore>,
) {
    // Wait until both waiters have announced themselves.
    ready.acquire();
    ready.acquire();

    // Hold both locks while setting the notification state and waking waiters.
    let mut data1 = m1.lock().unwrap();
    let mut data2 = m2.lock().unwrap();

    data1.notified = true;
    data2.notified = true;

    cv.for_w1.notify_all();
    cv.for_w2.notify_all();
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0#1897", WaiterData { notified: false }));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0#1964", WaiterData { notified: false }));
    let cv = Arc::new(Cv {
        for_w1: Condvar::new_named("for_w1#2054"),
        for_w2: Condvar::new_named("for_w2#2086"),
    });
    let ready = Arc::new(Semaphore::new());

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w1#2284", move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w2#2471", move || w2(m2, cv, ready))
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("notifier#2698", move || notifier(m1, m2, cv, ready))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
