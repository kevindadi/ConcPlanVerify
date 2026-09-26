use std::sync::{Arc, Mutex};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch1_tx: SyncSender<i32>,
    ch1_rx: Receiver<i32>,
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(0);
    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        ch1_tx: tx,
        ch1_rx: rx,
    });

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        {
            let _g = shared_s.m.lock().unwrap();
        }
        shared_s.ch1_tx.send(1).unwrap();
        {
            let _g = shared_s.m.lock().unwrap();
        }
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        let mut v: i32 = 0;
        {
            let _g = shared_r.m.lock().unwrap();
        }
        v = shared_r.ch1_rx.recv().unwrap();
        {
            let _g = shared_r.m.lock().unwrap();
        }
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
