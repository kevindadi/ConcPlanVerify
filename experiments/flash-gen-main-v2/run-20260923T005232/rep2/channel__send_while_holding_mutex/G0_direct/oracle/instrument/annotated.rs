mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Channel {
    slot: Mutex<Option<i32>>,
    ready: Condvar,
    taken: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            ready: Condvar::new(),
            taken: Condvar::new(),
        }
    }

    fn send(&self, value: i32) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.taken.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.ready.notify_one();
    }

    fn recv(&self) -> i32 {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.ready.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.taken.notify_one();
        value
    }
}

fn main() { cir_trace::init();
    let ch1 = Arc::new(Channel::new());
    let ch2 = Arc::new(Channel::new());

    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);

    let s = cir_trace::spawn("s", move || {
        ch1_s.send(1);
        let _ = ch2_s.recv();
    });

    let ch1_r = Arc::clone(&ch1);
    let ch2_r = Arc::clone(&ch2);

    let r = cir_trace::spawn("r", move || {
        let v = ch1_r.recv();
        ch2_r.send(v);
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
