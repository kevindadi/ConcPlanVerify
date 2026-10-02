mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

    fn post(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }

    fn wait(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }
}

// Each condition variable is paired with just one waiter's mutex.
struct WaitCvs {
    w1: Condvar,
    w2: Condvar,
}

struct Shared {
    m1: Mutex<bool>,
    m2: Mutex<bool>,
    cv: WaitCvs,
    ready: Semaphore,
}

fn w1(shared: Arc<Shared>) {
    let mut notified = shared.m1.lock().unwrap();
    shared.ready.post();

    while !*notified {
        notified = shared.cv.w1.wait(notified).unwrap();
    }
}

fn w2(shared: Arc<Shared>) {
    let mut notified = shared.m2.lock().unwrap();
    shared.ready.post();

    while !*notified {
        notified = shared.cv.w2.wait(notified).unwrap();
    }
}

fn notifier(shared: Arc<Shared>) {
    shared.ready.wait();
    shared.ready.wait();

    let mut notified1 = shared.m1.lock().unwrap();
    let mut notified2 = shared.m2.lock().unwrap();

    *notified1 = true;
    *notified2 = true;
    shared.cv.w1.notify_one();
    shared.cv.w2.notify_one();

    drop(notified2);
    drop(notified1);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m1: Mutex::new_named("m1#1598", false),
        m2: Mutex::new_named("m2#1629", false),
        cv: WaitCvs {
            w1: Condvar::new_named("w1#1688"),
            w2: Condvar::new_named("w2#1720"),
        },
        ready: Semaphore::new(),
    });

    let t1 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w1#1845", move || w1(shared))
    };
    let t2 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w2#1951", move || w2(shared))
    };
    let tn = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#2057", move || notifier(shared))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
