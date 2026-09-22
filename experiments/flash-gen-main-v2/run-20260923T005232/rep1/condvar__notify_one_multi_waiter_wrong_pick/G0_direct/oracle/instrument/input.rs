use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Semaphore implemented with Mutex + Condvar
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            cv: Condvar::new(),
        }
    }

    fn wait(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn signal(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cv.notify_one();
    }
}

fn main() {
    // Shared resources
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));

    // Shared state: number of waiters still blocked
    let waiters_blocked = Arc::new(Mutex::new(0usize));

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);
    let wb1 = Arc::clone(&waiters_blocked);

    let w1 = thread::spawn(move || {
        // Signal that this waiter is ready
        g12_1.signal();

        // Wait for notifier to be ready
        gN_1.wait();

        // Hold lock while waiting on cv
        let mut guard = m1.lock().unwrap();
        // Increment blocked count
        {
            let mut wb = wb1.lock().unwrap();
            *wb += 1;
        }
        // Wait until notified
        while *wb1.lock().unwrap() > 0 {
            guard = cv1.wait(guard).unwrap();
        }
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);
    let wb2 = Arc::clone(&waiters_blocked);

    let w2 = thread::spawn(move || {
        g12_2.signal();
        gN_2.wait();

        let mut guard = m2.lock().unwrap();
        {
            let mut wb = wb2.lock().unwrap();
            *wb += 1;
        }
        while *wb2.lock().unwrap() > 0 {
            guard = cv2.wait(guard).unwrap();
        }
        drop(guard);
    });

    let mN = Arc::clone(&m);
    let cvN = Arc::clone(&cv);
    let g12N = Arc::clone(&g12);
    let gNN = Arc::clone(&gN);
    let wbN = Arc::clone(&waiters_blocked);

    let notifier = thread::spawn(move || {
        // Wait for both waiters to be ready
        g12N.wait();
        g12N.wait();

        // Signal waiters that notifier is ready
        gNN.signal();
        gNN.signal();

        // Take lock before waking
        let mut guard = mN.lock().unwrap();

        // Wait until both waiters are actually blocked
        while *wbN.lock().unwrap() < 2 {
            guard = cvN.wait(guard).unwrap();
        }

        // Wake all waiters
        *wbN.lock().unwrap() = 0;
        cvN.notify_all();

        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
}
