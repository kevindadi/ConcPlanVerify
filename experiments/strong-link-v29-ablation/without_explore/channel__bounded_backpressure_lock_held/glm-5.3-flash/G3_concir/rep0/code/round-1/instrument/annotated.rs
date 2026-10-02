mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;
use std::thread;

fn sender(m: &Arc<Mutex<i32>>, ch_tx: &std::sync::mpsc::SyncSender<i32>) {
    // s1/s2: lock and unlock main::m
    let _guard = m.lock().unwrap();
    drop(_guard);

    // s3: send value 1
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();

    // s4/s5: lock and unlock main::m
    let _guard = m.lock().unwrap();
    drop(_guard);

    // s6: send value 2 (blocks until receiver has taken value 1,
    // since the channel holds at most one value)
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).unwrap();
}

fn receiver(m: &Arc<Mutex<i32>>, ch_rx: &std::sync::mpsc::Receiver<i32>) {
    // s1/s2: lock and unlock main::m
    let _guard = m.lock().unwrap();
    drop(_guard);

    // s3: receive first value (blocks while the channel is empty)
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();

    // s4/s5: lock and unlock main::m
    let _guard = m.lock().unwrap();
    drop(_guard);

    // s6: receive second value
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1017", 0));
    let (ch_tx, ch_rx) = sync_channel(1);

    let m_sender = Arc::clone(&m);
    let ch_tx_sender = ch_tx.clone();
    let sender_handle = crate::cir_trace::spawn("sender#1166", move || {
        sender(&m_sender, &ch_tx_sender);
    });

    let m_receiver = Arc::clone(&m);
    let receiver_handle = crate::cir_trace::spawn("receiver#1304", move || {
        receiver(&m_receiver, &ch_rx);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();
    drop(ch_tx);

    println!("DONE done=1");
 crate::cir_trace::finish();}
