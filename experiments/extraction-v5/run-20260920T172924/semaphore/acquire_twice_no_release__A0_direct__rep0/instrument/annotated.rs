mod cir_trace;
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
        cir_trace::ev(&cir_trace::tag_str(), "L1"); let mut c = self.n.lock().unwrap();
        while *c == 0 {
            cir_trace::ev(&cir_trace::tag_str(), "L2"); c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn rel(&self) {
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let mut c = self.n.lock().unwrap();
        *c += 1;
        cir_trace::ev(&cir_trace::tag_str(), "L4"); self.cv.notify_one();
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
