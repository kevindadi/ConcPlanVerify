use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

struct Channel {
    slot: Mutex<Option<u32>>,
    value_ready: Arc<Semaphore>,
    value_taken: Arc<Semaphore>,
}

fn s1(ch: Arc<Channel>, signal_ready: impl FnOnce()) {
    {
        let mut slot = ch.slot.lock().unwrap();
        *slot = Some(1);
    }
    signal_ready();
    let _ack = ch.value_taken.acquire();
}

fn r(ch: Arc<Channel>, signal_taken: impl FnOnce()) -> u32 {
    let _go = ch.value_ready.acquire();
    {
        let mut slot = ch.slot.lock().unwrap();
        slot.take().unwrap();
    }
    signal_taken();
    1
}

fn main() {
    let value_ready = Semaphore::new(1);
    let ready_token = value_ready.acquire();
    let value_taken = Semaphore::new(1);
    let taken_token = value_taken.acquire();

    let ch = Arc::new(Channel {
        slot: Mutex::new(None),
        value_ready,
        value_taken,
    });

    let ch_s = Arc::clone(&ch);
    let s1_task = thread::spawn(move || s1(ch_s, move || ready_token.release()));

    let ch_r = Arc::clone(&ch);
    let r_task = thread::spawn(move || r(ch_r, move || taken_token.release()));

    s1_task.join().unwrap();
    let done = r_task.join().unwrap();
    assert!(ch.slot.lock().unwrap().is_none());
    println!("DONE done={done}");
}
