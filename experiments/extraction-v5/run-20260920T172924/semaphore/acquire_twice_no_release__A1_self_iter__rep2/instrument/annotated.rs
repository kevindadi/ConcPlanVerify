mod cir_trace;
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
        cir_trace::ev(&cir_trace::tag_str(), "L1"); let mut c = self.n.lock().unwrap();
        // Reentrant: if this thread already owns the permit, just increment.
        if c.owner == Some(me) {
            c.count += 1;
            return;
        }
        while c.permits == 0 {
            cir_trace::ev(&cir_trace::tag_str(), "L2"); c = self.cv.wait(c).unwrap();
        }
        c.permits -= 1;
        c.owner = Some(me);
        c.count = 1;
    }

    fn rel(&self) {
        let me = std::thread::current().id();
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let mut c = self.n.lock().unwrap();
        if c.owner == Some(me) {
            c.count -= 1;
            if c.count == 0 {
                c.owner = None;
                c.permits += 1;
                cir_trace::ev(&cir_trace::tag_str(), "L4"); self.cv.notify_one();
            }
        } else {
            // Not the owner; just add a permit (defensive).
            c.permits += 1;
            cir_trace::ev(&cir_trace::tag_str(), "L5"); self.cv.notify_one();
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::sync_channel::<u32>(0);
    let s = std::sync::Arc::new(Sem::new(1));
    let s1 = std::sync::Arc::clone(&s);
    let tx1 = tx.clone();
    cir_trace::ev(&cir_trace::tag_str(), "L6"); let w1 = std::thread::spawn(move || {cir_trace::set_tag("tL6"); 
        s1.acq();
        s1.acq();
        s1.rel();
        s1.rel();
        let _ = tx1;
        let _ = rx;
    });
    let s2 = std::sync::Arc::clone(&s);
    cir_trace::ev(&cir_trace::tag_str(), "L7"); let w2 = std::thread::spawn(move || {cir_trace::set_tag("tL7"); 
        s2.acq();
        s2.rel();
    });
    cir_trace::ev(&cir_trace::tag_str(), "L8"); w1.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L9"); w2.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
