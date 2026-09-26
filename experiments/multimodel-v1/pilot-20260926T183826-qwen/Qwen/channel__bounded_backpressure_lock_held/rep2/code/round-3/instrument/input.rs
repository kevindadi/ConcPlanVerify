use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = thread::Builder::new().name("main::sender".to_string()).spawn(move || {
        // sender: channel_send ch 1
        tx.send(1).unwrap();
        // mutex_lock m
        let _guard = m_sender.lock().unwrap();
        // mutex_unlock m
        drop(_guard);
        // channel_send ch 2
        tx.send(2).unwrap();
        // done = 1 is local to sender, not printed directly here
    }).unwrap();

    let m_receiver = Arc::clone(&m);
    let receiver_handle = thread::Builder::new().name("main::receiver".to_string()).spawn(move || {
        // receiver: channel_recv ch -> v1
        let _v1 = rx.recv().unwrap();
        // mutex_lock m
        let _guard = m_receiver.lock().unwrap();
        // mutex_unlock m
        drop(_guard);
        // channel_recv ch -> v2
        let _v2 = rx.recv().unwrap();
    }).unwrap();

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
