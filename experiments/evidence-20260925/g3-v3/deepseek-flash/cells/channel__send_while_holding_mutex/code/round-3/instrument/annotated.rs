mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch1_tx: Mutex<Option<mpsc::SyncSender<i32>>>,
    ch1_rx: Mutex<Option<mpsc::Receiver<i32>>>,
}

fn main() { cir_trace::init();
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        ch1_tx: Mutex::new_named("shared_mutex1", Some(tx)),
        ch1_rx: Mutex::new_named("shared_mutex2", Some(rx)),
    });

    let shared_s = Arc::clone(&shared);
    let s = cir_trace::spawn("s", move || {
        {
            let _g = shared_s.m.lock().unwrap();
        }
        {
            let tx = shared_s.ch1_tx.lock().unwrap().take().unwrap();
            cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
        }
        {
            let _g = shared_s.m.lock().unwrap();
        }
    });

    let shared_r = Arc::clone(&shared);
    let r = cir_trace::spawn("r", move || {
        {
            let _g = shared_r.m.lock().unwrap();
        }
        let v: i32;
        {
            let rx = shared_r.ch1_rx.lock().unwrap().take().unwrap();
            cir_trace::record("channel_recv", "rx"); v = rx.recv().unwrap();
        }
        {
            let _g = shared_r.m.lock().unwrap();
        }
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
