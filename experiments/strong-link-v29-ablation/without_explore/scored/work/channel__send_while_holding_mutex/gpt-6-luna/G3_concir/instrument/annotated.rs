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

    crate::cir_trace::record("channel_recv", "ch1_rx"); let received = ch1_rx.recv().unwrap();
    crate::cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(received).unwrap();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#573", ()));

    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let s_m = Arc::clone(&m);
    let s_handle = crate::cir_trace::spawn("s#736", move || s(s_m, ch1_tx, ch2_rx));

    let r_m = Arc::clone(&m);
    let r_handle = crate::cir_trace::spawn("r#833", move || r(r_m, ch1_rx, ch2_tx));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
