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

// One shared condition-variable abstraction for waiters with separate data locks.
// The internal mutex provides a single mutex for the underlying Condvar.
struct Cv {
    state: Mutex<bool>,
    changed: Condvar,
}

impl Cv {
    fn new() -> Self {
        Self {
            state: Mutex::new(false),
            changed: Condvar::new(),
        }
    }

    fn wait<'a>(
        &self,
        data_lock: &'a Mutex<bool>,
        ready: &Semaphore,
    ) -> MutexGuard<'a, bool> {
        // Acquire locks in the same order used by notifier.
        let mut state = self.state.lock().unwrap();
        let data = data_lock.lock().unwrap();

        // Announce while holding the condition-variable mutex. The notifier
        // cannot proceed until this waiter has released its data lock and waits.
        ready.release();
        drop(data);

        while !*state {
            state = self.changed.wait(state).unwrap();
        }

        drop(state);
        data_lock.lock().unwrap()
    }
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let notified = cv.wait(&m1, &ready);
    assert!(*notified);
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: Arc<Semaphore>) {
    let notified = cv.wait(&m2, &ready);
    assert!(*notified);
}

fn notifier(
    m1: Arc<Mutex<bool>>,
    m2: Arc<Mutex<bool>>,
    cv: Arc<Cv>,
    ready: Arc<Semaphore>,
) {
    ready.acquire();
    ready.acquire();

    let mut state = cv.state.lock().unwrap();
    let mut notified1 = m1.lock().unwrap();
    let mut notified2 = m2.lock().unwrap();

    *notified1 = true;
    *notified2 = true;
    *state = true;
    cv.changed.notify_all();
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0#2327", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0#2369", false));
    let cv = Arc::new(Cv::new());
    let ready = Arc::new(Semaphore::new());

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w1#2599", move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("w2#2786", move || w2(m2, cv, ready))
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("notifier#3013", move || notifier(m1, m2, cv, ready))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
