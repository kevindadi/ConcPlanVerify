mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn s(m: Arc<Mutex<()>>, ch1_tx: SyncSender<i32>, ch2_rx: Receiver<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    crate::cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();
    crate::cir_trace::record("channel_recv", "ch2_rx"); let reply = ch2_rx.recv().unwrap();
    let _ = reply;
}

fn r(m: Arc<Mutex<()>>, ch1_rx: Receiver<i32>, ch2_tx: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    crate::cir_trace::record("channel_recv", "ch1_rx"); let message = ch1_rx.recv().unwrap();
    crate::cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(message).unwrap();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#571", ()));

    let (ch1_tx, ch1_rx) = sync_channel(0);
    let (ch2_tx, ch2_rx) = sync_channel(0);

    let s_m = Arc::clone(&m);
    let s_thread = crate::cir_trace::spawn("s#720", move || s(s_m, ch1_tx, ch2_rx));

    let r_m = Arc::clone(&m);
    let r_thread = crate::cir_trace::spawn("r#817", move || r(r_m, ch1_rx, ch2_tx));

    s_thread.join().unwrap();
    r_thread.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
