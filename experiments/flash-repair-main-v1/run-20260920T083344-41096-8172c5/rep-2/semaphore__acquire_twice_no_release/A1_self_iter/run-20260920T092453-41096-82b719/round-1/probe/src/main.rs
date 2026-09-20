use std::sync::mpsc;
use std::thread::ThreadId;

struct Sem {
    n: std::sync::Mutex<SemState>,
    cv: std::sync::Condvar,
}

struct SemState {
    // number of available permits (0 or 1)
    permits: u32,
    // current owner, if any
    owner: Option<ThreadId>,
    // how many times the owner has acquired
    count: u32,
}

impl Sem {
    fn new(n: u32) -> Self {
        Self {
            n: std::sync::Mutex::new(SemState {
                permits: n,
                owner: None,
                count: 0,
            }),
            cv: std::sync::Condvar::new(),
        }
    }

    fn acq(&self) {
        let me = std::thread::current().id();
        let mut c = self.n.lock().unwrap();
        // Reentrant: if this thread already owns the permit, just increment.
        if c.owner == Some(me) {
            c.count += 1;
            return;
        }
        while c.permits == 0 {
            c = self.cv.wait(c).unwrap();
        }
        c.permits -= 1;
        c.owner = Some(me);
        c.count = 1;
    }

    fn rel(&self) {
        let me = std::thread::current().id();
        let mut c = self.n.lock().unwrap();
        if c.owner == Some(me) {
            c.count -= 1;
            if c.count == 0 {
                c.owner = None;
                c.permits += 1;
                self.cv.notify_one();
            }
        } else {
            // Not the owner; just add a permit (defensive).
            c.permits += 1;
            self.cv.notify_one();
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::sync_channel::<u32>(0);
    let s = std::sync::Arc::new(Sem::new(1));
    let s1 = std::sync::Arc::clone(&s);
    let tx1 = tx.clone();
    let w1 = std::thread::spawn(move || {
        s1.acq();
        s1.acq();
        s1.rel();
        s1.rel();
        let _ = tx1;
        let _ = rx;
    });
    let s2 = std::sync::Arc::clone(&s);
    let w2 = std::thread::spawn(move || {
        s2.acq();
        s2.rel();
    });
    w1.join().unwrap();
    w2.join().unwrap();
    println!("DONE done=1");
}
