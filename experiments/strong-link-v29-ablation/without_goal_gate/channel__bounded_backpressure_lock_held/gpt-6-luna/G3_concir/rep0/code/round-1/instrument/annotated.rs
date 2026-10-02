mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch_tx: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();

    let guard = m.lock().unwrap();
    drop(guard);

    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch_rx: Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();

    let guard = m.lock().unwrap();
    drop(guard);

    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#549", ()));
    let (ch_tx, ch_rx) = sync_channel(1);

    let sender_m = Arc::clone(&m);
    let receiver_m = Arc::clone(&m);

    let sender_handle = crate::cir_trace::spawn("sender#699", move || sender(sender_m, ch_tx));
    let receiver_handle = crate::cir_trace::spawn("receiver#773", move || receiver(receiver_m, ch_rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
