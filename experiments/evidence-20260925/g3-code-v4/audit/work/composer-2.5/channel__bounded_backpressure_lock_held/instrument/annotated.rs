mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn sender(tx: std::sync::mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
}

fn receiver(rx: std::sync::mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    cir_trace::record("channel_recv", "rx"); let v1 = rx.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "rx"); let v2 = rx.recv().unwrap();
    let _ = (v1, v2);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = sync_channel(1);

    thread::scope(|s| {
        s.spawn({
            let m = Arc::clone(&m);
            move || sender(tx, m)
        });
        s.spawn({
            let m = Arc::clone(&m);
            move || receiver(rx, m)
        });
    });

    println!("DONE done=1");
 cir_trace::finish();}
