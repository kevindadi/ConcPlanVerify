mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch1_tx: SyncSender<i32>,
    ch1_rx: Receiver<i32>,
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i32>(0);
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        ch1_tx: tx,
        ch1_rx: rx,
    });

    let shared_s = Arc::clone(&shared);
    let s = cir_trace::spawn("s", move || {
        {
            let _g = shared_s.m.lock().unwrap();
        }
        cir_trace::record("channel_send", "channel"); shared_s.ch1_tx.send(1).unwrap();
        {
            let _g = shared_s.m.lock().unwrap();
        }
    });

    let shared_r = Arc::clone(&shared);
    let r = cir_trace::spawn("r", move || {
        let mut v: i32 = 0;
        {
            let _g = shared_r.m.lock().unwrap();
        }
        cir_trace::record("channel_recv", "channel"); v = shared_r.ch1_rx.recv().unwrap();
        {
            let _g = shared_r.m.lock().unwrap();
        }
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
