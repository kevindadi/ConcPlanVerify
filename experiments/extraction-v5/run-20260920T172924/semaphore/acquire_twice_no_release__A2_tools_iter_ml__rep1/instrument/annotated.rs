mod cir_trace;
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
        cir_trace::ev(&cir_trace::tag_str(), "L1"); let mut s = self.state.lock().unwrap();
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
            cir_trace::ev(&cir_trace::tag_str(), "L2"); s = self.cv.wait(s).unwrap();
        }
    }

    fn rel(&self) {
        let me = std::thread::current().id();
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let mut s = self.state.lock().unwrap();
        assert_eq!(s.owner, Some(me), "release by non-owner");
        assert!(s.count > 0, "release without acquire");
        s.count -= 1;
        if s.count == 0 {
            s.owner = None;
            s.available += 1;
            cir_trace::ev(&cir_trace::tag_str(), "L4"); self.cv.notify_one();
        }
    }
}

fn main() {
    let s = Arc::new(Sem::new(1));

    let s1 = Arc::clone(&s);
    cir_trace::ev(&cir_trace::tag_str(), "L5"); let w1 = std::thread::spawn(move || {cir_trace::set_tag("tL5"); 
        s1.acq();
        s1.acq();
        s1.rel();
        s1.rel();
    });

    let s2 = Arc::clone(&s);
    cir_trace::ev(&cir_trace::tag_str(), "L6"); let w2 = std::thread::spawn(move || {cir_trace::set_tag("tL6"); 
        s2.acq();
        s2.rel();
    });

    cir_trace::ev(&cir_trace::tag_str(), "L7"); w1.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L8"); w2.join().unwrap();

    println!("DONE done=1");
cir_trace::finish(); }
