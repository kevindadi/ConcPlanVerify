mod cir_trace;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;

struct Sem {
    state: Mutex<SemState>,
    cv: Condvar,
}

struct SemState {
    owner: Option<ThreadId>,
    count: u32, // permits held by owner
}

impl Sem {
    fn new() -> Self {
        Self {
            state: Mutex::new(SemState {
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
            match s.owner {
                None => {
                    s.owner = Some(me);
                    s.count = 1;
                    return;
                }
                Some(o) if o == me => {
                    s.count += 1;
                    return;
                }
                _ => {
                    cir_trace::ev(&cir_trace::tag_str(), "L2"); s = self.cv.wait(s).unwrap();
                }
            }
        }
    }

    fn rel(&self) {
        let me = std::thread::current().id();
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let mut s = self.state.lock().unwrap();
        if s.owner == Some(me) {
            s.count -= 1;
            if s.count == 0 {
                s.owner = None;
                cir_trace::ev(&cir_trace::tag_str(), "L4"); self.cv.notify_one();
            }
        }
    }
}

fn main() {
    let s = Arc::new(Sem::new());

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
