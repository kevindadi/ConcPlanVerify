use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            cv: Condvar::new(),
        }
    }

    fn post(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cv.notify_one();
    }

    fn wait(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }
}

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    g12.post();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    drop(proceed);
    gN.wait();
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    g12.post();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    drop(proceed);
    gN.wait();
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    g12.wait();
    g12.wait();

    {
        let mut proceed = m.lock().unwrap();
        *proceed = true;
        cv.notify_all();
    }

    gN.post();
    gN.post();
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));

    let w1_handle = {
        let (m, cv, g12, gN) = (m.clone(), cv.clone(), g12.clone(), gN.clone());
        thread::spawn(move || w1(m, cv, g12, gN))
    };
    let w2_handle = {
        let (m, cv, g12, gN) = (m.clone(), cv.clone(), g12.clone(), gN.clone());
        thread::spawn(move || w2(m, cv, g12, gN))
    };
    let notifier_handle = {
        let (m, cv, g12, gN) = (m.clone(), cv.clone(), g12.clone(), gN.clone());
        thread::spawn(move || notifier(m, cv, g12, gN))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
}
