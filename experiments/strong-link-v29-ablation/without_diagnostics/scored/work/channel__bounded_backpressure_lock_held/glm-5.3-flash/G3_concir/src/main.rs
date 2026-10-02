mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn sender(ch_tx: std::sync::mpsc::SyncSender<i32>, m: Arc<Mutex<i32>>) {
    // send value 1 (blocks if the one-slot channel is full)
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();
    // send value 2 (waits until the receiver has taken the first value)
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).unwrap();
    // lock m, update done, unlock m
    let mut done = m.lock().unwrap();
    *done += 1;
    drop(done);
}

fn receiver(ch_rx: std::sync::mpsc::Receiver<i32>, m: Arc<Mutex<i32>>) {
    // take value 1 (blocks if the channel is empty)
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
    // take value 2
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
    // lock m, update done, unlock m
    let mut done = m.lock().unwrap();
    *done += 1;
    drop(done);
}

fn main() { crate::cir_trace::init();
    // one-slot channel: capacity 1
    let (ch_tx, ch_rx) = sync_channel(1);
    // shared lock m guarding the shared var done
    let m = Arc::new(Mutex::new_named("m_mutex0#949", 0));

    let m_sender = Arc::clone(&m);
    let sender_handle = crate::cir_trace::spawn("sender#1018", move || sender(ch_tx, m_sender));

    let m_receiver = Arc::clone(&m);
    let receiver_handle = crate::cir_trace::spawn("receiver#1130", move || receiver(ch_rx, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
