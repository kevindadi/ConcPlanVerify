mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Channel {
    slot: Mutex<Option<i32>>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn send(&self, value: i32) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.not_full.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> i32 {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.not_empty.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.not_full.notify_one();
        value
    }
}

fn sender(m: Arc<Mutex<()>>, ch: Arc<Channel>) {
    let _ = m.lock().unwrap();
    cir_trace::record("channel_send", "ch"); ch.send(1);

    let _ = m.lock().unwrap();
    cir_trace::record("channel_send", "ch"); ch.send(2);

    let _ = m.lock().unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Arc<Channel>) {
    cir_trace::record("channel_recv", "ch"); let _v1 = ch.recv();

    let _ = m.lock().unwrap();
    cir_trace::record("channel_recv", "ch"); let _v2 = ch.recv();

    let _ = m.lock().unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1252", ()));
    let ch = Arc::new(Channel::new());

    let m_s = Arc::clone(&m);
    let ch_s = Arc::clone(&ch);
    let sender_handle = cir_trace::spawn("sender#1388", move || sender(m_s, ch_s));

    let m_r = Arc::clone(&m);
    let ch_r = Arc::clone(&ch);
    let receiver_handle = cir_trace::spawn("receiver#1519", move || receiver(m_r, ch_r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
