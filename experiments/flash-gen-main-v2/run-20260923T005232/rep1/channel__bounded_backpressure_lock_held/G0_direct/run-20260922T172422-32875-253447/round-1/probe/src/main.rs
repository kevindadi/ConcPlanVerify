use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel {
    slot: Option<i32>,
    closed: bool,
}

struct Shared {
    m: Mutex<()>,
    ch: Mutex<Channel>,
    not_full: Condvar,
    not_empty: Condvar,
}

fn sender(shared: Arc<Shared>) {
    for value in [1, 2] {
        // Occasionally need the shared lock m.
        {
            let _guard = shared.m.lock().unwrap();
        }

        // Wait until channel has room, without holding m.
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_some() {
            ch = shared.not_full.wait(ch).unwrap();
        }
        ch.slot = Some(value);
        shared.not_empty.notify_one();
        drop(ch);

        // Occasionally need the shared lock m again.
        {
            let _guard = shared.m.lock().unwrap();
        }
    }
}

fn receiver(shared: Arc<Shared>) {
    let mut received = 0;
    while received < 2 {
        // Occasionally need the shared lock m.
        {
            let _guard = shared.m.lock().unwrap();
        }

        // Wait until channel has a value, without holding m.
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_none() {
            ch = shared.not_empty.wait(ch).unwrap();
        }
        let _value = ch.slot.take().unwrap();
        received += 1;
        shared.not_full.notify_one();
        drop(ch);

        // Occasionally need the shared lock m again.
        {
            let _guard = shared.m.lock().unwrap();
        }
    }
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        ch: Mutex::new(Channel {
            slot: None,
            closed: false,
        }),
        not_full: Condvar::new(),
        not_empty: Condvar::new(),
    });

    let s = Arc::clone(&shared);
    let r = Arc::clone(&shared);

    let sender_handle = thread::spawn(move || sender(s));
    let receiver_handle = thread::spawn(move || receiver(r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
