use std::sync::{Arc, Condvar, Mutex};

struct Sem {
    n: Mutex<u32>,
    cv: Condvar,
}

impl Sem {
    fn new(n: u32) -> Self {
        Self {
            n: Mutex::new(n),
            cv: Condvar::new(),
        }
    }

    fn acq(&self) {
        let mut c = self.n.lock().unwrap();
        while *c == 0 {
            c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn rel(&self) {
        let mut c = self.n.lock().unwrap();
        *c += 1;
        drop(c);
        self.cv.notify_one();
    }
}

fn main() {
    let s = Arc::new(Sem::new(1));

    let s1 = Arc::clone(&s);
    let w1 = std::thread::spawn(move || {
        // Worker 1 acquires the permit twice. The semaphore has only one
        // permit, so it must not hold the permit across the second acquire
        // (that would deadlock). Instead it acquires and releases the permit
        // once, then acquires and releases it again, releasing it exactly as
        // many times as it acquired before returning.
        s1.acq();
        s1.rel();
        s1.acq();
        s1.rel();
    });

    let s2 = Arc::clone(&s);
    let w2 = std::thread::spawn(move || {
        s2.acq();
        s2.rel();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
