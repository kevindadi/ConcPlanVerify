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
        let mut s = self.state.lock().unwrap();
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
                    s = self.cv.wait(s).unwrap();
                }
            }
        }
    }

    fn rel(&self) {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap();
        if s.owner == Some(me) {
            s.count -= 1;
            if s.count == 0 {
                s.owner = None;
                self.cv.notify_one();
            }
        }
    }
}

fn main() {
    let s = Arc::new(Sem::new());

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
