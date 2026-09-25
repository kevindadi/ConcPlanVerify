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
    let cv = Arc::new(Condvar::new());
    let ready = Arc::new(Semaphore::new(0));

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let ready_w1 = Arc::clone(&ready);

    let w1 = thread::spawn(move || {
        let mut guard = m1_w1.lock().unwrap();
        ready_w1.signal();
        while !*guard {
            guard = cv_w1.wait(guard).unwrap();
        }
        *guard = true;
    });

    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let ready_w2 = Arc::clone(&ready);

    let w2 = thread::spawn(move || {
        let mut guard = m2_w2.lock().unwrap();
        ready_w2.signal();
        while !*guard {
            guard = cv_w2.wait(guard).unwrap();
        }
        *guard = true;
    });

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ready_n = Arc::clone(&ready);

    let notifier = thread::spawn(move || {
        ready_n.wait();
        ready_n.wait();

        {
            let mut g1 = m1_n.lock().unwrap();
            *g1 = true;
            cv_n.notify_all();
        }
        {
            let mut g2 = m2_n.lock().unwrap();
            *g2 = true;
            cv_n.notify_all();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
}
