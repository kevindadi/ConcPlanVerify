mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;

fn sender(m: &Arc<Mutex<()>>, ch_tx: &std::sync::mpsc::SyncSender<i32>) {
    let _guard = m.lock().unwrap();
    drop(_guard);
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).unwrap();
}

fn receiver(ch_rx: &std::sync::mpsc::Receiver<i32>, m: &Arc<Mutex<()>>) {
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
    let _guard = m.lock().unwrap();
    drop(_guard);
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#493", ()));
    let (ch_tx, ch_rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let tx = ch_tx.clone();
    let sender_handle = crate::cir_trace::spawn("sender#640", move || {
        sender(&m_sender, &tx);
    });

    let m_receiver = Arc::clone(&m);
    let receiver_handle = crate::cir_trace::spawn("receiver#773", move || {
        receiver(&ch_rx, &m_receiver);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
