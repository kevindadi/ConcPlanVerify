mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    mutex: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            mutex: Mutex::new(count),
            condvar: Condvar::new(),
        }
    }

    fn wait(&self) {
        let mut count = self.mutex.lock().unwrap();
        while *count == 0 {
            count = self.condvar.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn signal(&self) {
        let mut count = self.mutex.lock().unwrap();
        *count += 1;
        self.condvar.notify_one();
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);

    let w1 = cir_trace::spawn("w1", move || {
        g12_1.signal();
        gN_1.wait();
        let mut guard = m1.lock().unwrap();
        while *guard == 0 {
            guard = cv1.wait(guard).unwrap();
        }
        *guard -= 1;
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);

    let w2 = cir_trace::spawn("w2", move || {
        g12_2.signal();
        gN_2.wait();
        let mut guard = m2.lock().unwrap();
        while *guard == 0 {
            guard = cv2.wait(guard).unwrap();
        }
        *guard -= 1;
    });

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_3 = Arc::clone(&g12);
    let gN_3 = Arc::clone(&gN);

    let notifier = cir_trace::spawn("notifier", move || {
        g12_3.wait();
        g12_3.wait();
        let mut guard = m3.lock().unwrap();
        *guard += 2;
        cv3.notify_all();
        drop(guard);
        gN_3.signal();
        gN_3.signal();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
