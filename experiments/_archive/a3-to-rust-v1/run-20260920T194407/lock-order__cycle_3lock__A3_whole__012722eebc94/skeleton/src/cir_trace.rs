// Generated cir_trace runtime (std only).
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

thread_local! {
    static TAG: std::cell::RefCell<String> = std::cell::RefCell::new("t0".to_string());
}

pub fn tag_str() -> String {
    TAG.with(|t| t.borrow().clone())
}

pub fn set_tag(tag: &str) {
    TAG.with(|t| *t.borrow_mut() = tag.to_string());
}

static EVENTS: OnceLock<Mutex<Vec<(String, String)>>> = OnceLock::new();

pub fn ev(tag: &str, sid: &str) {
    let m = EVENTS.get_or_init(|| Mutex::new(Vec::new()));
    m.lock().unwrap().push((tag.to_string(), sid.to_string()));
}

pub fn finish() {
    if let Ok(path) = std::env::var("CIR_TRACE_OUT") {
        let m = EVENTS.get_or_init(|| Mutex::new(Vec::new()));
        let guard = m.lock().unwrap();
        let mut out = String::new();
        for (t, s) in guard.iter() {
            out.push_str(&format!("{{\"t\":\"{}\",\"sid\":\"{}\"}}\n", t, s));
        }
        let _ = std::fs::write(path, out);
    }
}

pub struct Semaphore {
    count: Mutex<i64>,
    cv: Condvar,
}

impl Semaphore {
    pub fn new(n: i64) -> Arc<Self> {
        Arc::new(Semaphore { count: Mutex::new(n), cv: Condvar::new() })
    }
    pub fn acquire(&self, n: i64) {
        let mut c = self.count.lock().unwrap();
        while *c < n {
            c = self.cv.wait(c).unwrap();
        }
        *c -= n;
    }
    pub fn release(&self, n: i64) {
        let mut c = self.count.lock().unwrap();
        *c += n;
        self.cv.notify_all();
    }
}

#[allow(dead_code)]
pub struct Channel<T> {
    buffer: Mutex<VecDeque<T>>,
    cap: usize,
    send_cv: Condvar,
    recv_cv: Condvar,
}

impl<T: Send> Channel<T> {
    pub fn new(cap: usize) -> Arc<Self> {
        Arc::new(Channel {
            buffer: Mutex::new(VecDeque::new()),
            cap,
            send_cv: Condvar::new(),
            recv_cv: Condvar::new(),
        })
    }
    pub fn send(&self, v: T) {
        let mut b = self.buffer.lock().unwrap();
        while self.cap != 0 && b.len() >= self.cap {
            b = self.send_cv.wait(b).unwrap();
        }
        if self.cap == 0 {
            b.push_back(v);
            self.recv_cv.notify_one();
            while !b.is_empty() {
                b = self.send_cv.wait(b).unwrap();
            }
        } else {
            b.push_back(v);
            self.recv_cv.notify_one();
        }
    }
    pub fn recv(&self) -> T {
        let mut b = self.buffer.lock().unwrap();
        while b.is_empty() {
            b = self.recv_cv.wait(b).unwrap();
        }
        let v = b.pop_front().unwrap();
        self.send_cv.notify_one();
        v
    }
}
