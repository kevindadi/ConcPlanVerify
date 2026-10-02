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

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }
}

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    g12.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    drop(proceed);
    gN.release();
}

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    waiter(m, cv, g12, gN);
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    waiter(m, cv, g12, gN);
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>) {
    g12.acquire();
    g12.acquire();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1349", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1393"));
    let g12 = Arc::new(Semaphore::new());
    let gN = Arc::new(Semaphore::new());

    let w1_handle = {
        let (m, cv, g12, gN) = (m.clone(), cv.clone(), g12.clone(), gN.clone());
        cir_trace::spawn("w1#1596", move || w1(m, cv, g12, gN))
    };
    let w2_handle = {
        let (m, cv, g12, gN) = (m.clone(), cv.clone(), g12.clone(), gN.clone());
        cir_trace::spawn("w2#1756", move || w2(m, cv, g12, gN))
    };
    let notifier_handle = {
        let (m, cv, g12) = (m.clone(), cv.clone(), g12.clone());
        cir_trace::spawn("notifier#1906", move || notifier(m, cv, g12))
    };

    gN.acquire();
    gN.acquire();

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
