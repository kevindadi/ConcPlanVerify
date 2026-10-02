mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;
use std::thread;

fn sender(ch_tx: std::sync::mpsc::SyncSender<i32>, m: &Arc<Mutex<i32>>) {
    // s1: channel_send ch 1
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).expect("send failed");

    // s2..s5: lock m, read taken, seen = seen + 1, unlock m
    let mut guard = m.lock().expect("lock poisoned");
    let taken = *guard;
    let seen = taken + 1;
    let _ = seen;
    drop(guard);

    // s6: channel_send ch 2 (blocks while the one-slot channel is full)
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).expect("send failed");

    // s7: return
}

fn receiver(ch_rx: std::sync::mpsc::Receiver<i32>, m: &Arc<Mutex<i32>>) {
    // s1: channel_recv ch -> v
    crate::cir_trace::record("channel_recv", "ch_rx"); let v = ch_rx.recv().expect("recv failed");
    let _ = v;

    // s2..s4: lock m, taken = taken + 1, unlock m
    {
        let mut guard = m.lock().expect("lock poisoned");
        *guard = *guard + 1;
    }

    // s5: channel_recv ch -> _
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().expect("recv failed");

    // s6..s8: lock m, taken = taken + 1, unlock m
    {
        let mut guard = m.lock().expect("lock poisoned");
        *guard = *guard + 1;
    }

    // s9: return
}

fn main() { crate::cir_trace::init();
    // ch: bounded channel with capacity 1
    let (ch_tx, ch_rx) = sync_channel::<i32>(1);
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#1287", 0)); // guards var `taken`

    let m_sender = Arc::clone(&m);
    let sender_handle = crate::cir_trace::spawn("sender#1378", move || {
        sender(ch_tx, &m_sender);
    });

    let m_receiver = Arc::clone(&m);
    let receiver_handle = crate::cir_trace::spawn("receiver#1508", move || {
        receiver(ch_rx, &m_receiver);
    });

    sender_handle.join().expect("sender panicked");
    receiver_handle.join().expect("receiver panicked");

    // After joins, read shared state only to print the terminal line.
    let taken = *m.lock().expect("lock poisoned");
    let done = if taken == 2 { 1 } else { 0 };
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
