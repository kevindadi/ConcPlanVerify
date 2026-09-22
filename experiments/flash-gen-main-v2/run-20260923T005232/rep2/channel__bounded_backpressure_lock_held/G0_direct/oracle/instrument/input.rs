use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel {
    slot: Option<i32>,
    done: bool,
}

struct Shared {
    m: Mutex<()>,
    ch: Mutex<Channel>,
    not_empty: Condvar,
    not_full: Condvar,
}

fn sender(shared: Arc<Shared>) {
    for value in [1, 2] {
        // Acquire the shared lock m occasionally, but never while waiting on ch.
        {
            let _guard = shared.m.lock().unwrap();
        }

        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_some() {
            ch = shared.not_full.wait(ch).unwrap();
        }
        ch.slot = Some(value);
        shared.not_empty.notify_one();
        drop(ch);

        {
            let _guard = shared.m.lock().unwrap();
        }
    }
}

fn receiver(shared: Arc<Shared>) {
    let mut received = 0;
    while received < 2 {
        {
            let _guard = shared.m.lock().unwrap();
        }

        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_none() {
            ch = shared.not_empty.wait(ch).unwrap();
        }
        let _value = ch.slot.take().unwrap();
        received += 1;
        shared.not_full.notify_one();
        drop(ch);

        {
            let _guard = shared.m.lock().unwrap();
        }
    }

    let mut ch = shared.ch.lock().unwrap();
    ch.done = true;
    shared.not_empty.notify_all();
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        ch: Mutex::new(Channel {
            slot: None,
            done: false,
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
