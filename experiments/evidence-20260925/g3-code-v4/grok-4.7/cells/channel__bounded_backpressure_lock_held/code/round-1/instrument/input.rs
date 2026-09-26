use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let ch = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let ch_tx = ch.0;
    let sender_thread = thread::spawn(move || {
        sender(m_sender, ch_tx);
    });

    let m_receiver = Arc::clone(&m);
    let ch_rx = ch.1;
    let receiver_thread = thread::spawn(move || {
        receiver(m_receiver, ch_rx);
    });

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();
    println!("DONE done=1");
}

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(1).unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    let mut v: i32 = 0;
    let guard = m.lock().unwrap();
    drop(guard);
    v = ch.recv().unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    v = ch.recv().unwrap();
    let _ = v;
}
