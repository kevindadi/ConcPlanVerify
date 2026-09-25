mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Channel {
    slot: Option<i32>,
    closed: bool,
}

struct Shared {
    m: Mutex<()>,
    ch: Mutex<Channel>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl Shared {
    fn new() -> Self {
        Shared {
            m: Mutex::new(()),
            ch: Mutex::new(Channel {
                slot: None,
                closed: false,
            }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn send(&self, value: i32) {
        let mut ch = self.ch.lock().unwrap();
        while ch.slot.is_some() {
            ch = self.not_full.wait(ch).unwrap();
        }
        ch.slot = Some(value);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> i32 {
        let mut ch = self.ch.lock().unwrap();
        while ch.slot.is_none() {
            ch = self.not_empty.wait(ch).unwrap();
        }
        let value = ch.slot.take().unwrap();
        self.not_full.notify_one();
        value
    }
}

fn sender(shared: Arc<Shared>) {
    // Occasionally need the shared lock m, but never while waiting on the channel.
    {
        let _guard = shared.m.lock().unwrap();
    }

    cir_trace::record("channel_send", "shared"); shared.send(1);

    {
        let _guard = shared.m.lock().unwrap();
    }

    cir_trace::record("channel_send", "shared"); shared.send(2);
}

fn receiver(shared: Arc<Shared>) {
    {
        let _guard = shared.m.lock().unwrap();
    }

    cir_trace::record("channel_recv", "shared"); let _first = shared.recv();

    {
        let _guard = shared.m.lock().unwrap();
    }

    cir_trace::record("channel_recv", "shared"); let _second = shared.recv();
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared::new());

    let s = Arc::clone(&shared);
    let r = Arc::clone(&shared);

    let sender_handle = cir_trace::spawn("sender", move || sender(s));
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
