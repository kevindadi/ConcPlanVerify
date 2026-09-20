use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;

struct Sem {
    state: Mutex<SemState>,
    cv: Condvar,
}

struct SemState {
    // Number of permits currently available (0 or 1).
    available: u32,
    // Owner of the permit, if held.
    owner: Option<ThreadId>,
    // Re-entrant acquisition count for the owner.
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
            if s.owner == Some(me) {
                // Re-entrant acquire: already own the permit.
                s.count += 1;
                return;
            }
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
        assert_eq!(s.owner, Some(me), "release by non-owner");
        assert!(s.count > 0, "release without acquire");
        s.count -= 1;
        if s.count == 0 {
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
