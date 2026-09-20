use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;

struct Sem {
    state: Mutex<SemState>,
    cv: Condvar,
}

struct SemState {
    // Number of available permits (0 or 1 for this binary semaphore).
    available: u32,
    // The thread currently holding the permit, if any.
    owner: Option<ThreadId>,
    // How many times the owner has acquired the permit.
    count: u32,
}

impl Sem {
    fn new(n: u32) -> Self {
        Self {
            state: Mutex::new(SemState {
                available: n,
                owner: None,
                count: 0,
            }),
            cv: Condvar::new(),
        }
    }

    fn acq(&self) {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap();
        loop {
            // Reentrant acquisition: if this thread already owns the permit,
            // just bump the recursion count.
            if s.owner == Some(me) {
                s.count += 1;
                return;
            }
            // Otherwise, take the permit if it is available.
            if s.available > 0 {
                s.available -= 1;
                s.owner = Some(me);
                s.count = 1;
                return;
            }
            s = self.cv.wait(s).unwrap();
        }
    }

    fn rel(&self) {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap();
        // Only the owner may release.
        if s.owner != Some(me) {
            return;
        }
        s.count -= 1;
        if s.count == 0 {
            // Fully released: free the permit and wake a waiter.
            s.owner = None;
            s.available += 1;
            self.cv.notify_one();
        }
    }
}

fn main() {
    let s = Arc::new(Sem::new(1));

    let s1 = Arc::clone(&s);
    let w1 = std::thread::spawn(move || {
        s1.acq();
        s1.acq();
        s1.rel();
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
