use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    mutex: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            mutex: Mutex::new(count),
            cv: Condvar::new(),
        }
    }

    fn wait(&self) {
        let mut count = self.mutex.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn signal(&self) {
        let mut count = self.mutex.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

fn main() {
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));
    // Each waiter needs its own condition variable paired with its own lock.
    let cv1 = Arc::new(Condvar::new());
    let cv2 = Arc::new(Condvar::new());
    let ready = Arc::new(Semaphore::new(0));

    let m1_w1 = Arc::clone(&m1);
    let cv1_w1 = Arc::clone(&cv1);
    let ready_w1 = Arc::clone(&ready);

    let w1 = thread::spawn(move || {
        let mut guard = m1_w1.lock().unwrap();
        ready_w1.signal();
        while !*guard {
            guard = cv1_w1.wait(guard).unwrap();
        }
        *guard = true;
    });

    let m2_w2 = Arc::clone(&m2);
    let cv2_w2 = Arc::clone(&cv2);
    let ready_w2 = Arc::clone(&ready);

    let w2 = thread::spawn(move || {
        let mut guard = m2_w2.lock().unwrap();
        ready_w2.signal();
        while !*guard {
            guard = cv2_w2.wait(guard).unwrap();
        }
        *guard = true;
    });

    let m1_notifier = Arc::clone(&m1);
    let m2_notifier = Arc::clone(&m2);
    let cv1_notifier = Arc::clone(&cv1);
    let cv2_notifier = Arc::clone(&cv2);
    let ready_notifier = Arc::clone(&ready);

    let notifier = thread::spawn(move || {
        // Wait until both waiters have announced themselves.
        ready_notifier.wait();
        ready_notifier.wait();

        // Hold each lock the waiters need in order to wake and finish.
        let mut g1 = m1_notifier.lock().unwrap();
        let mut g2 = m2_notifier.lock().unwrap();

        *g1 = true;
        *g2 = true;

        // Wake each waiter on its own condition variable.
        cv1_notifier.notify_all();
        cv2_notifier.notify_all();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
}
