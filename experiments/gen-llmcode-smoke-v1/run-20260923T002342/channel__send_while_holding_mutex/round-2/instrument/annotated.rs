mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

struct Shared {
    lock: Mutex<()>,
    done: Mutex<i32>,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        lock: Mutex::new_named("shared_mutex0", ()),
        done: Mutex::new_named("shared_mutex1", 0),
    });

    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let shared_s = Arc::clone(&shared);
    let s = cir_trace::spawn("s", move || {
        {
            let _g = shared_s.lock.lock().unwrap();
        }
        tx.send(1).unwrap();
        {
            let _g = shared_s.lock.lock().unwrap();
        }
    });

    let shared_r = Arc::clone(&shared);
    let r = cir_trace::spawn("r", move || {
        {
            let _g = shared_r.lock.lock().unwrap();
        }
        let _v: i32 = rx.recv().unwrap();
        {
            let _g = shared_r.lock.lock().unwrap();
        }
        {
            let mut d = shared_r.done.lock().unwrap();
            *d = 1;
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    let done = *shared.done.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
