mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn s(shared_lock: Arc<Mutex<()>>, ch1_tx: SyncSender<i32>, ch2_rx: Receiver<i32>) {
    {
        let _guard = shared_lock.lock().unwrap();
    }
    crate::cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();
    crate::cir_trace::record("channel_recv", "ch2_rx"); let _ = ch2_rx.recv().unwrap();
}

fn r(shared_lock: Arc<Mutex<()>>, ch1_rx: Receiver<i32>, ch2_tx: SyncSender<i32>) {
    {
        let _guard = shared_lock.lock().unwrap();
    }
    crate::cir_trace::record("channel_recv", "ch1_rx"); let _ = ch1_rx.recv().unwrap();
    crate::cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(2).unwrap();
}

fn main() { crate::cir_trace::init();
    let shared_lock = Arc::new(Mutex::new_named("shared_lock_mutex0#584", ()));
    let (ch1_tx, ch1_rx) = sync_channel(0);
    let (ch2_tx, ch2_rx) = sync_channel(0);

    let s_lock = Arc::clone(&shared_lock);
    let s_thread = crate::cir_trace::spawn("s#745", move || s(s_lock, ch1_tx, ch2_rx));

    let r_thread = crate::cir_trace::spawn("r#815", move || r(shared_lock, ch1_rx, ch2_tx));

    s_thread.join().unwrap();
    r_thread.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
