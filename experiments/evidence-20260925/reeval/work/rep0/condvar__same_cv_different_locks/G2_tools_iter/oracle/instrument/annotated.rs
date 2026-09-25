mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let ready = Arc::new(Semaphore::new(0));

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let ready_w1 = Arc::clone(&ready);

    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let ready_w2 = Arc::clone(&ready);

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ready_n = Arc::clone(&ready);

    let w1 = cir_trace::spawn("w1", move || {
        let mut guard = m1_w1.lock().unwrap();
        ready_w1.signal();
        while !*guard {
            guard = cv_w1.wait(guard).unwrap();
        }
        *guard = false;
    });

    let w2 = cir_trace::spawn("w2", move || {
        let mut guard = m2_w2.lock().unwrap();
        ready_w2.signal();
        while !*guard {
            guard = cv_w2.wait(guard).unwrap();
        }
        *guard = false;
    });

    let notifier = cir_trace::spawn("notifier", move || {
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
 cir_trace::finish();}
