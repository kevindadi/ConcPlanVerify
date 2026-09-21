use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, ThreadId};

struct Sem {
    inner: Mutex<SemState>,
    cv: Condvar,
}

struct SemState {
    n: u32,                 // number of real permits available
    owner: Option<ThreadId>, // thread currently holding the real permit
    depth: u32,             // reentrant acquisition depth
}

impl Sem {
    fn new(n: u32) -> Self {
        Self {
            inner: Mutex::new(SemState {
                n,
                owner: None,
                depth: 0,
            }),
            cv: Condvar::new(),
        }
    }

    fn acq(&self) {
        let tid = thread::current().id();
        let mut g = self.inner.lock().unwrap();
        loop {
            if g.n > 0 {
                // Acquire the one real permit.
                g.n -= 1;
                g.owner = Some(tid);
                g.depth = 1;
                return;
            } else if g.owner == Some(tid) {
                // Same thread: recursive acquisition.
                g.depth += 1;
                return;
            } else {
                g = self.cv.wait(g).unwrap();
            }
        }
    }

    fn rel(&self) {
        let tid = thread::current().id();
        let mut g = self.inner.lock().unwrap();
        assert_eq!(g.owner, Some(tid), "release by non-owner");
        if g.depth > 1 {
            // Still holding the real permit.
            g.depth -= 1;
        } else {
            // Release the real permit.
            g.owner = None;
            g.depth = 0;
            g.n += 1;
            self.cv.notify_one();
        }
    }
}

fn main() {
    let s = Arc::new(Sem::new(1));
    let s1 = Arc::clone(&s);
    let w1 = thread::spawn(move || {
        s1.acq();
        s1.acq();
        s1.rel();
        s1.rel();
    });

    let s2 = Arc::clone(&s);
    let w2 = thread::spawn(move || {
        s2.acq();
        s2.rel();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    println!("DONE done=1");
}
