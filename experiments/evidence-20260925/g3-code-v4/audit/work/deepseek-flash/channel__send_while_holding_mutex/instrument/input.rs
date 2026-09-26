use std::sync::{mpsc, Arc, Mutex};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch1_tx: Mutex<Option<mpsc::SyncSender<i32>>>,
    ch1_rx: Mutex<Option<mpsc::Receiver<i32>>>,
}

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        ch1_tx: Mutex::new(Some(tx)),
        ch1_rx: Mutex::new(Some(rx)),
    });

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        {
            let _g = shared_s.m.lock().unwrap();
        }
        {
            let tx = shared_s.ch1_tx.lock().unwrap().take().unwrap();
            tx.send(1).unwrap();
        }
        {
            let _g = shared_s.m.lock().unwrap();
        }
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        {
            let _g = shared_r.m.lock().unwrap();
        }
        let v: i32;
        {
            let rx = shared_r.ch1_rx.lock().unwrap().take().unwrap();
            v = rx.recv().unwrap();
        }
        {
            let _g = shared_r.m.lock().unwrap();
        }
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
