mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Channel<T> {
    slot: Option<T>,
    closed: bool,
}

struct Chan<T> {
    state: Mutex<Channel<T>>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl<T> Chan<T> {
    fn new() -> Self {
        Chan {
            state: Mutex::new(Channel { slot: None, closed: false }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn send(&self, value: T) {
        let mut guard = self.state.lock().unwrap();
        while guard.slot.is_some() {
            guard = self.not_full.wait(guard).unwrap();
        }
        guard.slot = Some(value);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> T {
        let mut guard = self.state.lock().unwrap();
        while guard.slot.is_none() {
            guard = self.not_empty.wait(guard).unwrap();
        }
        let value = guard.slot.take().unwrap();
        self.not_full.notify_one();
        value
    }
}

fn main() { cir_trace::init();
    let ch: Arc<Chan<i32>> = Arc::new(Chan::new());
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let ch_s = Arc::clone(&ch);
    let m_s = Arc::clone(&m);
    let sender = cir_trace::spawn("sender", move || {
        // First value: take lock briefly, then send without holding it.
        {
            let _g = m_s.lock().unwrap();
        }
        ch_s.send(1);

        // Second value: take lock briefly, then send without holding it.
        {
            let _g = m_s.lock().unwrap();
        }
        ch_s.send(2);
    });

    let ch_r = Arc::clone(&ch);
    let m_r = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver", move || {
        let a = ch_r.recv();
        {
            let _g = m_r.lock().unwrap();
        }
        let b = ch_r.recv();
        {
            let _g = m_r.lock().unwrap();
        }
        (a, b)
    });

    sender.join().unwrap();
    let (a, b) = receiver.join().unwrap();
    println!("DONE done={}", (a + b) / 3);
 cir_trace::finish();}
