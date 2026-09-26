mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender_handle", move || {
        // sender: channel_send ch 1
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
        // mutex_lock m
        let _guard = m_sender.lock().unwrap();
        // mutex_unlock m
        drop(_guard);
        // channel_send ch 2
        cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
        // done = 1 is local to sender, not printed directly here
    });

    let m_receiver = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver_handle", move || {
        // receiver: channel_recv ch -> v1
        cir_trace::record("channel_recv", "rx"); let _v1 = rx.recv().unwrap();
        // mutex_lock m
        let _guard = m_receiver.lock().unwrap();
        // mutex_unlock m
        drop(_guard);
        // channel_recv ch -> v2
        cir_trace::record("channel_recv", "rx"); let _v2 = rx.recv().unwrap();
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
