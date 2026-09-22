mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Channel {
    slot: Option<i32>,
    closed: bool,
}

struct Shared {
    m: Mutex<Channel>,
    cv: Condvar,
}

fn sender(shared: Arc<Shared>) {
    let values = [1, 2];
    for v in values {
        // Acquire the shared lock m, then wait until the channel has room.
        let mut ch = shared.m.lock().unwrap();
        while ch.slot.is_some() {
            ch = shared.cv.wait(ch).unwrap();
        }
        // Channel is empty; place the value.
        ch.slot = Some(v);
        shared.cv.notify_all();
        // Release the lock before doing anything else.
        drop(ch);
    }
}

fn receiver(shared: Arc<Shared>) {
    let mut taken = 0;
    while taken < 2 {
        let mut ch = shared.m.lock().unwrap();
        while ch.slot.is_none() {
            ch = shared.cv.wait(ch).unwrap();
        }
        let _v = ch.slot.take().unwrap();
        taken += 1;
        shared.cv.notify_all();
        drop(ch);
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", Channel {
            slot: None,
            closed: false,
        }),
        cv: Condvar::new_named("shared_condvar0"),
    });

    let s = Arc::clone(&shared);
    let r = Arc::clone(&shared);

    let sender_handle = cir_trace::spawn("sender_handle", move || sender(s));
    let receiver_handle = cir_trace::spawn("receiver_handle", move || receiver(r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
