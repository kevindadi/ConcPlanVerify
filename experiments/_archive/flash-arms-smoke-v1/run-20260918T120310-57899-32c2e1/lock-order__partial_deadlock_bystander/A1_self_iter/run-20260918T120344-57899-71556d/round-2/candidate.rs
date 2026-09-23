use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::Duration;

struct Handshake {
    // semaphore-like state: 0 = not signaled, 1 = signaled
    state: Mutex<bool>,
    cv: Condvar,
}

impl Handshake {
    fn new() -> Self {
        Handshake {
            state: Mutex::new(false),
            cv: Condvar::new(),
        }
    }

    fn signal(&self) {
        let mut s = self.state.lock().unwrap();
        *s = true;
        self.cv.notify_one();
    }

    fn wait(&self) {
        let mut s = self.state.lock().unwrap();
        while !*s {
            s = self.cv.wait(s).unwrap();
        }
    }
}

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let hs = Arc::new(Handshake::new());

    let m1a = Arc::clone(&m1);
    let m2a = Arc::clone(&m2);
    let hsa = Arc::clone(&hs);

    let a = thread::spawn(move || {
        let _g1 = m1a.lock().unwrap();
        hsa.signal();
        hsa.wait();
        let _g2 = m2a.lock().unwrap();
        // critical section
    });

    let m1b = Arc::clone(&m1);
    let m2b = Arc::clone(&m2);
    let hsb = Arc::clone(&hs);

    let b = thread::spawn(move || {
        let _g2 = m2b.lock().unwrap();
        hsb.signal();
        hsb.wait();
        let _g1 = m1b.lock().unwrap();
        // critical section
    });

    let bystander = thread::spawn(|| loop {
        thread::sleep(Duration::from_millis(10));
    });

    a.join().unwrap();
    b.join().unwrap();
    // bystander runs forever; don't join it
    let _ = bystander;
}
