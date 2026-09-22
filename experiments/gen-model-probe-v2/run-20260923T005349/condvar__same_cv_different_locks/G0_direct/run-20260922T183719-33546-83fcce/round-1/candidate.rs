use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A simple counting semaphore built from a mutex and a condition variable.
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            count: Mutex::new(initial),
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

/// Shared resources: one lock per waiter, one shared condition variable,
/// and the `ready` semaphore used by waiters to announce themselves.
struct Shared {
    m1: Mutex<bool>,
    m2: Mutex<bool>,
    cv: Condvar,
    ready: Semaphore,
}

fn w1(shared: Arc<Shared>) {
    // R4: hold our own lock while waiting.
    let mut notified = shared.m1.lock().unwrap();
    // R5: announce that we are about to wait (while still holding m1, so the
    // notifier cannot acquire m1 until we are actually blocked on cv).
    shared.ready.post();
    while !*notified {
        notified = shared.cv.wait(notified).unwrap();
    }
    // Proceed after notification; lock released on drop.
}

fn w2(shared: Arc<Shared>) {
    // R4: hold our own lock while waiting.
    let mut notified = shared.m2.lock().unwrap();
    // R5: announce that we are about to wait (while still holding m2).
    shared.ready.post();
    while !*notified {
        notified = shared.cv.wait(notified).unwrap();
    }
    // Proceed after notification; lock released on drop.
}

fn notifier(shared: Arc<Shared>) {
    // R6: wait until both waiters have announced themselves.
    shared.ready.wait();
    shared.ready.wait();

    // R7/R8: hold each lock a waiter needs in order to wake and finish.
    // Because each waiter announced while holding its lock, acquiring the
    // lock here guarantees that waiter is already blocked on `cv`, so the
    // notification can never be missed and no waiter is left blocked.
    let mut n1 = shared.m1.lock().unwrap();
    *n1 = true;
    let mut n2 = shared.m2.lock().unwrap();
    *n2 = true;
    shared.cv.notify_all();
}

fn main() {
    let shared = Arc::new(Shared {
        m1: Mutex::new(false),
        m2: Mutex::new(false),
        cv: Condvar::new(),
        ready: Semaphore::new(0),
    });

    // R1: start both waiters and the notifier at the same time.
    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));
    let h3 = thread::spawn(move || notifier(s3));

    // R9: every role finishes; join them all.
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    // R10: print exactly this line and exit.
    println!("DONE done=1");
}
