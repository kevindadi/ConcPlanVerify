use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        ch: Mutex::new(Channel {
            slot: None,
            closed: false,
        }),
        not_empty: Condvar::new(),
        not_full: Condvar::new(),
    });

    let s = Arc::clone(&shared);
    let r = Arc::clone(&shared);

    let sender_handle = thread::spawn(move || sender(s));
    let receiver_handle = thread::spawn(move || receiver(r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
