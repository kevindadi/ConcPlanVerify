use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(m: Arc<Mutex<i32>>, ch: SyncSender<i32>) {
    // mutex_lock main::m
    {
        let _guard = m.lock().unwrap();
        // assign_local sent = 0
        let sent: i32 = 0;
        let _ = sent;
    } // mutex_unlock main::m

    // channel_send main::ch value 1
    ch.send(1).unwrap();
    // channel_send main::ch value 2 (blocks until receiver takes the first value)
    ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<i32>>, ch: Receiver<i32>) {
    // channel_recv main::ch dst v1
    let v1: i32 = ch.recv().unwrap();

    // mutex_lock main::m
    {
        let _guard = m.lock().unwrap();
        // assign_local v2 = v1
        let mut v2: i32 = 0;
        v2 = v1;
        let _ = v2;
    } // mutex_unlock main::m

    // channel_recv main::ch dst v2
    let v2: i32 = ch.recv().unwrap();
    let _ = v2;
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    // channel main::ch with capacity 1
    let (tx, rx) = sync_channel::<i32>(2);

    let m_sender = Arc::clone(&m);
    let tx_sender = tx.clone();
    let h_sender = thread::spawn(move || {
        sender(m_sender, tx_sender);
    });

    let m_receiver = Arc::clone(&m);
    let h_receiver = thread::spawn(move || {
        receiver(m_receiver, rx);
    });

    // join h_sender; join h_receiver
    h_sender.join().unwrap();
    h_receiver.join().unwrap();

    // terminal line
    println!("DONE done=1");
}
