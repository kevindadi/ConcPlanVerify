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

fn sender(shared: Arc<Shared>) {
    // Occasionally need the shared lock m.
    {
        let _guard = shared.m.lock().unwrap();
    }

    // Send first value.
    {
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_some() {
            ch = shared.not_full.wait(ch).unwrap();
        }
        ch.slot = Some(1);
        shared.not_empty.notify_one();
    }

    // Occasionally need the shared lock m again.
    {
        let _guard = shared.m.lock().unwrap();
    }

    // Send second value.
    {
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_some() {
            ch = shared.not_full.wait(ch).unwrap();
        }
        ch.slot = Some(2);
        shared.not_empty.notify_one();
    }
}

fn receiver(shared: Arc<Shared>) {
    // Occasionally need the shared lock m.
    {
        let _guard = shared.m.lock().unwrap();
    }

    // Receive first value.
    {
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_none() {
            ch = shared.not_empty.wait(ch).unwrap();
        }
        let _v = ch.slot.take().unwrap();
        shared.not_full.notify_one();
    }

    // Occasionally need the shared lock m again.
    {
        let _guard = shared.m.lock().unwrap();
    }

    // Receive second value.
    {
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_none() {
            ch = shared.not_empty.wait(ch).unwrap();
        }
        let _v = ch.slot.take().unwrap();
        shared.not_full.notify_one();
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        ch: Mutex::new_named("shared_mutex1", Channel {
            slot: None,
            closed: false,
        }),
        not_empty: Condvar::new_named("shared_condvar0"),
        not_full: Condvar::new_named("shared_condvar1"),
    });

    let s = Arc::clone(&shared);
    let r = Arc::clone(&shared);

    let sender_handle = cir_trace::spawn("sender", move || sender(s));
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
