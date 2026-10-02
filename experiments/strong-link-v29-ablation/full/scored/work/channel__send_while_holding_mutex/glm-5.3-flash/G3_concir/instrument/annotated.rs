mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;
use std::thread;

fn s(lock: &Arc<Mutex<()>>, ch1_tx: std::sync::mpsc::SyncSender<i32>, ch2_rx: std::sync::mpsc::Receiver<i32>) {
    let _g = lock.lock().unwrap();
    drop(_g);
    crate::cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();
    crate::cir_trace::record("channel_recv", "ch2_rx"); let _ = ch2_rx.recv().unwrap();
    let _g = lock.lock().unwrap();
    drop(_g);
}

fn r(lock: &Arc<Mutex<()>>, ch1_rx: std::sync::mpsc::Receiver<i32>, ch2_tx: std::sync::mpsc::SyncSender<i32>) {
    let _g = lock.lock().unwrap();
    drop(_g);
    crate::cir_trace::record("channel_recv", "ch1_rx"); let _ = ch1_rx.recv().unwrap();
    crate::cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(1).unwrap();
    let _g = lock.lock().unwrap();
    drop(_g);
}

fn main() { crate::cir_trace::init();
    let lock = Arc::new(Mutex::new_named("lock_mutex0#681", ()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let hs = crate::cir_trace::spawn("s#843", move || {
        s(&lock_s, ch1_tx, ch2_rx);
    });

    let lock_r = Arc::clone(&lock);
    let hr = crate::cir_trace::spawn("r#961", move || {
        r(&lock_r, ch1_rx, ch2_tx);
    });

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
