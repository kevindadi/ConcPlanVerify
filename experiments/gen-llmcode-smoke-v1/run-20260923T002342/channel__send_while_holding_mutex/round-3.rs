use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

struct Shared {
    lock: Mutex<()>,
    done: Mutex<i32>,
}

fn main() {
    let shared = Arc::new(Shared {
        lock: Mutex::new(()),
        done: Mutex::new(0),
    });

    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let shared_s = Arc::clone(&shared);
    let shared_r = Arc::clone(&shared);

    let s_handle = thread::spawn(move || {
        {
            let _g = shared_s.lock.lock().unwrap();
        }
        tx.send(1).unwrap();
        {
            let _g = shared_s.lock.lock().unwrap();
        }
    });

    let r_handle = thread::spawn(move || {
        {
            let _g = shared_r.lock.lock().unwrap();
        }
        let _v = rx.recv().unwrap();
        {
            let _g = shared_r.lock.lock().unwrap();
        }
        {
            let mut d = shared_r.done.lock().unwrap();
            *d = 1;
        }
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    let done = *shared.done.lock().unwrap();
    println!("DONE done={}", done);
}
