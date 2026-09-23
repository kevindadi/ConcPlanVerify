struct SemState {
    n: u32,
    owner: Option<ThreadId>,
    rec: u32,
}

struct Sem {
    state: Mutex<SemState>,
    cv: Condvar,
}

impl Sem {
    fn new(n: u32) -> Self {
        Self {
            state: Mutex::new(SemState { n, owner: None, rec: 0 }),
            cv: Condvar::new(),
        }
    }
    fn acq(&self) {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap();
        if s.owner == Some(me) {
            s.rec += 1;
            return;
        }
        while s.n == 0 {
            s = self.cv.wait(s).unwrap();
        }
        s.n -= 1;
        s.owner = Some(me);
        s.rec = 1;
    }
    fn rel(&self) {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap();
        if s.owner != Some(me) {
            // not owner; shouldn't happen per contract
            return;
        }
        s.rec -= 1;
        if s.rec == 0 {
            s.owner = None;
            s.n += 1;
            self.cv.notify_one();
        }
    }
}
