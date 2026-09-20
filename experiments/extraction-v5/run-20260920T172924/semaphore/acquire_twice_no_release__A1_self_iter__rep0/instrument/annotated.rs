mod cir_trace;
use std::sync::mpsc;
use std::thread::ThreadId;

struct Sem {
    state: std::sync::Mutex<SemState>,
    cv: std::sync::Condvar,
}

struct SemState {
    // None means the permit is free.
    owner: Option<ThreadId>,
    // Recursion depth for the current owner.
    count: u32,
}

impl Sem {
    fn new() -> Self {
        Self {
            state: std::sync::Mutex::new(SemState {
                owner: None,
                count: 0,
            }),
            cv: std::sync::Condvar::new(),
        }
    }

    fn acq(&self) {
        let me = std::thread::current().id();
        cir_trace::ev(&cir_trace::tag_str(), "L1"); let mut st = self.state.lock().unwrap();
        // Reentrant acquisition: same thread already owns the permit.
        if st.owner == Some(me) {
            st.count += 1;
            return;
        }
        // Wait until the permit is free.
        while st.owner.is_some() {
            cir_trace::ev(&cir_trace::tag_str(), "L2"); st = self.cv.wait(st).unwrap();
        }
        st.owner = Some(me);
        st.count = 1;
    }

    fn rel(&self) {
        let me = std::thread::current().id();
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let mut st = self.state.lock().unwrap();
        // Only the owner may release.
        if st.owner != Some(me) {
            return;
        }
        st.count -= 1;
        if st.count == 0 {
            st.owner = None;
            cir_trace::ev(&cir_trace::tag_str(), "L4"); self.cv.notify_one();
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::sync_channel::<u32>(0);
    let s = std::sync::Arc::new(Sem::new());
    let s1 = std::sync::Arc::clone(&s);
    let tx1 = tx.clone();
    cir_trace::ev(&cir_trace::tag_str(), "L5"); let w1 = std::thread::spawn(move || {cir_trace::set_tag("tL5"); 
        s1.acq();
        s1.acq();
        s1.rel();
        s1.rel();
        let _ = tx1;
        let _ = rx;
    });
    let s2 = std::sync::Arc::clone(&s);
    cir_trace::ev(&cir_trace::tag_str(), "L6"); let w2 = std::thread::spawn(move || {cir_trace::set_tag("tL6"); 
        s2.acq();
        s2.rel();
    });
    cir_trace::ev(&cir_trace::tag_str(), "L7"); w1.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L8"); w2.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
